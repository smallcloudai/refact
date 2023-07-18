from typing import Optional, Tuple

import torch
from torch import nn

from refact_models.codify_modules import MultiheadSelfAttention
from refact_models.codify_modules import Block, BlockCheckpointing
from refact_models.checkpoint_loader import load_config, load_checkpoint
from refact_models.generation import generate
from refact_models.lora import LoraMixin


class CodifyModel(nn.Module, LoraMixin):
    def __init__(self, config, device: str):
        super().__init__()
        self.device = device
        self.config = config
        self.embed_dim = config.E
        n_vocab_align64 = (config.n_vocab + 63) // 64 * 64
        self.wte = self.to_device(nn.Embedding(n_vocab_align64, self.embed_dim))
        self.blocks = nn.Sequential(*[self.to_device(Block(config)) for _ in range(config.L)])

        # lm model
        self.ln_f = self.to_device(nn.LayerNorm(self.embed_dim))
        self.lm_head = self.to_device(nn.Linear(config.E, n_vocab_align64, bias=False))

        # highlight model
        self.bidir_sa_ln = self.to_device(nn.LayerNorm(config.E))
        self.bidir_sa = self.to_device(MultiheadSelfAttention(config))
        self.bidir_2logits_ln = self.to_device(nn.LayerNorm(config.E))
        self.bidir_2logits = self.to_device(nn.Linear(config.E, 3))

        self.cache_dir: Optional[str] = None
        self.model_name: Optional[str] = None

    def to_device(self, module: nn.Module):
        module = module.to(self.device)
        if self.device.startswith("cuda"):
            module = module.to(torch.half)
        return module

    @property
    def encoding(self):
        return self.config.encoding

    @classmethod
    def from_pretrained(cls, path: str, device: str = "cuda", repo_id: Optional[str] = None):
        config = load_config(path, repo_id)
        model = cls(config, device)
        model = load_checkpoint(model, path, repo_id)
        return model

    def generate(self, *args, **kwargs):
        return generate(self, *args, **kwargs)

    def forward(self, x: torch.Tensor,
                attention_mask: Optional[torch.Tensor],
                past_key_values: Optional[Tuple[Tuple[torch.Tensor]]] = None,
                use_cache: Optional[bool] = False):

        hidden_states = self.wte(x)

        presents = () if use_cache else None
        if past_key_values is None:
            past_key_values = tuple([None] * len(self.blocks))

        for i, (block, layer_past) in enumerate(zip(self.blocks, past_key_values)):
            hidden_states, present = block(
                hidden_states=hidden_states,
                attention_mask=attention_mask,
                layer_past=layer_past,
                use_cache=use_cache)
            if use_cache:
                presents = presents + (present,)

        return hidden_states, presents

    def lm_forward(self, hidden_states):
        hidden_states_normed = self.ln_f(hidden_states)
        hidden_states_norm = hidden_states_normed / 2.0
        logits = self.lm_head(hidden_states_norm)
        return logits

    def forward_train_cp(self, x: torch.Tensor) -> torch.Tensor:
        from torch.utils.checkpoint import checkpoint_sequential

        hidden_states = self.wte(x)
        with BlockCheckpointing(self.blocks):
            hidden_states = checkpoint_sequential(self.blocks, len(self.blocks), hidden_states.requires_grad_(True))
        logits = torch.utils.checkpoint.checkpoint(self.lm_forward, hidden_states.requires_grad_(True))

        return logits

    def highlight_forward(self, x_bte, first_bt, diffhlpoint):
        B, T, E = x_bte.shape
        assert E == self.embed_dim
        # assert T == self.hps.T, (T, self.hps.T)   # for testing, can be smaller 1024
        assert T == first_bt.shape[1], str(first_bt.shape)
        assert T == diffhlpoint.shape[1], str(diffhlpoint.shape)
        mask_BTT = torch.zeros((B, T, T), device=x_bte.device, dtype=torch.bool)
        for t in range(T):
            mask_BTT[:, t, t] = True
        for b in range(B):
            # first_bt[b]       # [0,0,0,1,0,0,0,0,0]
            # diffhlpoint[b]    # [0,0,0,0,0,0,1,0,0]
            # first nonzero in first_bt
            t1s = (first_bt[b] == 1).nonzero(as_tuple=False).squeeze(1)
            t2s = (diffhlpoint[b] == 1).nonzero(as_tuple=False).squeeze(1)
            assert len(t1s) >= len(t2s)
            for t1, t2 in zip(t1s, t2s):
                t1 = t1.item()
                t2 = t2.item()
                assert t1 < t2
                if t2 < T:
                    t2 += 1
                # fill rectangle on main diagonal
                mask_BTT[b, t1:t2, t1:t2] = 1
        inside, _state = self.bidir_sa.forward(self.bidir_sa_ln(x_bte), mask_BTT, None)
        return inside
