import torch
import torch.nn as nn
import torch.nn.functional as F


class LayerNormNoBias(nn.Module):

    def __init__(self, shape: int, eps: float = 1e-5):
        super().__init__()
        self.shape = (shape,)
        self.eps = eps
        self.weight = nn.Parameter(torch.empty(self.shape))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return F.layer_norm(x, self.shape, self.weight, None, self.eps)