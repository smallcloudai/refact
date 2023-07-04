import torch
import torch.nn as nn
import torch.nn.functional as F

from refact_models.alibias import ALiBiBias

from typing import Optional, Tuple


class MultiHeadAttention(nn.Module):

    def __init__(self, config):
        super().__init__()

        self.E = config.E
        self.attn_heads = config.attn_heads
        self.head_dim = self.E // self.attn_heads
        self.kv_attn_heads = 1
        self.scale = self.head_dim ** -0.5
        self.alibibias = ALiBiBias(config)

        self.q = nn.Linear(self.E, self.E, bias=False)
        self.k = nn.Linear(self.E, self.head_dim, bias=False)
        self.v = nn.Linear(self.E, self.head_dim, bias=False)
        self.out = nn.Linear(self.E, self.E, bias=False)

    def _attention(self,
                   query: torch.Tensor,
                   key: torch.Tensor,
                   value: torch.Tensor,
                   attention_mask: Optional[torch.Tensor]) -> torch.Tensor:
        attn_weights = torch.matmul(query * self.scale, key.transpose(-1, -2))
        alibi = self.alibibias(
            query.shape[0], query.shape[2], key.shape[2],
            query.device, query.dtype)
        attn_weights = attn_weights + alibi
        if attention_mask is not None:
            attn_weights = torch.masked_fill(attn_weights, attention_mask, -10000)

        attn_weights = F.softmax(attn_weights, dim=-1)
        out = torch.matmul(attn_weights, value)
        return out

    def forward(self,
                x: torch.Tensor,
                attention_mask: Optional[torch.Tensor] = None,
                layer_past: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
                use_cache: bool = False) -> Tuple[torch.Tensor, Optional[Tuple[torch.Tensor, torch.Tensor]]]:
        b, t, _ = x.shape

        query = self.q(x)
        key = self.k(x)
        value = self.v(x)

        query = query.view(b, t, self.attn_heads, self.head_dim).permute(0, 2, 1, 3)
        key = key.view(b, t, self.kv_attn_heads, self.head_dim).permute(0, 2, 1, 3)
        value = value.view(b, t, self.kv_attn_heads, self.head_dim).permute(0, 2, 1, 3)

        if layer_past is not None:
            past_key, past_value = layer_past
            key = torch.cat((past_key, key), dim=-2)
            value = torch.cat((past_value, value), dim=-2)

        if use_cache is True:
            present = (key, value)
        else:
            present = None

        attn_output = self._attention(query, key, value, attention_mask=attention_mask)
        attn_output = attn_output.permute(0, 2, 1, 3).contiguous()
        attn_output = attn_output.view(b, t, -1)

        attn_output = self.out(attn_output)
        return attn_output, present
