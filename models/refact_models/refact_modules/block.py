import torch
import torch.nn as nn

from code_contrast.modeling.refact_modules import LayerNormNoBias
from code_contrast.modeling.refact_modules import MultiHeadAttention
from code_contrast.modeling.refact_modules import MLP

from typing import Tuple, Optional


class Block(nn.Module):

    def __init__(self, config):
        super().__init__()
        self.ln_a = LayerNormNoBias(config.E)
        self.mha = MultiHeadAttention(config)
        self.ln_m = LayerNormNoBias(config.E)
        self.pw = MLP(config.E)

    def forward(self,
                hidden_states: torch.Tensor,
                **mha_kwargs) -> Tuple[torch.Tensor, Optional[Tuple[torch.Tensor, torch.Tensor]]]:
        attn_output, present = self.mha(self.ln_a(hidden_states), **mha_kwargs)
        mix = attn_output + hidden_states
        attn_mix = self.pw(self.ln_m(mix))
        output = attn_mix + mix
        return output, present
