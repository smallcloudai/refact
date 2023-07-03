import torch
import torch.nn as nn

from code_contrast.modeling.refact_modules import LayerNormNoBias
from code_contrast.modeling.refact_modules import Unembedding


class Final(nn.Module):

    def __init__(self, config):
        super().__init__()
        self.ln = LayerNormNoBias(config.E)
        self.unemb = Unembedding(config)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x_normed = self.ln(x)
        logits = self.unemb(x_normed)
        return logits