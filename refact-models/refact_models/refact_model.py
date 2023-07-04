from typing import Optional, Tuple

import torch
import torch.nn as nn

from refact_models.lora import LoraMixin
from refact_models.refact_modules import Embedding
from refact_models.refact_modules import Block
from refact_models.refact_modules import Final

from refact_models.checkpoint_loader import load_config
from refact_models.checkpoint_loader import load_checkpoint
from refact_models.generation import generate


class RefactModel(nn.Module, LoraMixin):

    def __init__(self, config, device: str):
        super().__init__()
        self.device = device
        self.config = config

        self.emb = self.to_device(Embedding(config))
        self.layers = nn.Sequential(*[self.to_device(Block(config)) for _ in range(config.L)])
        self.final = self.to_device(Final(config))

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
        hidden_states = self.emb(x)

        presents = () if use_cache else None
        if past_key_values is None:
            past_key_values = tuple([None] * len(self.layers))

        for i, (block, layer_past) in enumerate(zip(self.layers, past_key_values)):
            hidden_states, present = block(hidden_states=hidden_states,
                                           attention_mask=attention_mask,
                                           layer_past=layer_past,
                                           use_cache=use_cache)
            if use_cache:
                presents = presents + (present,)

        return hidden_states, presents

    def lm_forward(self, hidden_states):
        return self.final(hidden_states)
