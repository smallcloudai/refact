import torch
import torch.nn as nn


def _align(n_vocab: int, align: int = 64) -> int:
    return (n_vocab + align - 1) // align * align


class Embedding(nn.Module):

    def __init__(self, config):
        super().__init__()
        self.layer = nn.Embedding(_align(config.n_vocab), config.E)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.layer(x)


class Unembedding(nn.Module):

    def __init__(self, config):
        super().__init__()
        self.layer = nn.Linear(config.E, _align(config.n_vocab), bias=False)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.layer(x)
