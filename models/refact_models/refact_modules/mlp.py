import torch
import torch.nn as nn
import torch.nn.functional as F


class MLP(nn.Module):

    def __init__(self, embed_dim, mult: float = 4.0, multiple_of: int = 256):
        super().__init__()
        hidden_dim = embed_dim * mult
        hidden_dim = int(2 * hidden_dim / 3)
        hidden_dim = multiple_of * ((hidden_dim + multiple_of - 1) // multiple_of)
        self.linear_1 = nn.Linear(embed_dim, hidden_dim, bias=False)
        self.linear_2 = nn.Linear(hidden_dim, embed_dim, bias=False)
        self.linear_3 = nn.Linear(embed_dim, hidden_dim, bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x1 = F.silu(self.linear_1(x))
        x2 = self.linear_3(x)
        x = self.linear_2(x1 * x2)
        return x