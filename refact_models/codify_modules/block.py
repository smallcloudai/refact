from typing import Optional, Tuple

import torch
from torch import nn

from refact_models.codify_modules import MultiheadSelfAttention
from refact_models.codify_modules import MLP


class BlockCheckpointing:
    def __init__(self, blocks):
        self.blocks = blocks

    def __enter__(self):
        for b in self.blocks:
            b.forward_single_output = True

    def __exit__(self, exc_type, exc_val, exc_tb):
        for b in self.blocks:
            b.forward_single_output = False


class Block(nn.Module):
    def __init__(self, config,
                 residual_scale: float = 1.0):
        super().__init__()
        self.ln_a = nn.LayerNorm(config.E)
        self.ln_m = nn.LayerNorm(config.E)

        self.sa = MultiheadSelfAttention(config)
        self.mlp = MLP(config)

        self.forward_single_output = False
        # self.residual_scale = residual_scale
        # self.use_residual_scale = config.use_res_scale

    def forward(self,
                hidden_states: torch.Tensor,
                attention_mask: Optional[torch.Tensor] = None,
                layer_past: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
                use_cache: bool = False):
        residual = hidden_states
        hidden_states_norm = self.ln_a(hidden_states)

        attn_output, present = self.sa(hidden_states_norm,
                                       attention_mask=attention_mask,
                                       layer_past=layer_past,
                                       use_cache=use_cache)

        mix = attn_output + residual
        norm_mix = self.ln_m(mix)
        feed_forward_hidden_states = self.mlp(norm_mix)

        output = residual + (attn_output + feed_forward_hidden_states)

        if self.forward_single_output:
            return output
        else:
            return output, present
