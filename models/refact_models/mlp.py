import math

import torch
from torch import nn

_kAlpha = math.sqrt(2.0 / math.pi)


def gelu(x):
    return 0.5 * x * (1 + torch.tanh(_kAlpha * (x + 0.044715 * x * x * x)))


class MLP(nn.Module):
    def __init__(self, config):
        super(MLP, self).__init__()
        self.ln_1 = nn.Linear(config.E, config.E * config.mlp_mult)
        self.ln_2 = nn.Linear(config.E * config.mlp_mult, config.E)

    def forward(self, x: torch.Tensor):
        x = self.ln_1(x)
        x = gelu(x)
        x = self.ln_2(x)
        return x
