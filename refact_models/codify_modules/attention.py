from typing import Optional, Tuple

import torch
import torch.nn.functional as F
from torch import nn

from refact_models.codify_modules.alibias import ALiBiBias


class MultiheadSelfAttention(nn.Module):
    def __init__(self, config):
        super(MultiheadSelfAttention, self).__init__()
        self.qkv = nn.Linear(config.E, 3 * config.E)
        self.out = nn.Linear(config.E, config.E)

        self.num_heads = config.attn_heads
        self.dim_head = config.E // self.num_heads
        self.alibias = ALiBiBias(config)
        self.scale = 8 / self.dim_head
        # if False:
        #     self.scale = config.mup_scale / self.dim_head
        # else:
        #     self.scale = self.dim_head ** -0.5

    def _split_heads(self, tensor):
        new_shape = tensor.shape[:-1] + (self.num_heads, self.dim_head)
        tensor = tensor.view(new_shape)
        return tensor.permute(0, 2, 1, 3)

    def _merge_heads(self, tensor):
        tensor = tensor.permute(0, 2, 1, 3).contiguous()
        return tensor.view(*tensor.shape[:-2], -1)

    def attention(self,
                  query: torch.Tensor,
                  key: torch.Tensor,
                  value: torch.Tensor,
                  attention_mask: Optional[torch.Tensor]):
        attn_weights = torch.matmul(query * self.scale, key.transpose(-1, -2))
        alibi = self.alibias(query.shape[0],
                             query.shape[2],
                             key.shape[2],
                             query.device,
                             query.dtype)
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
                use_cache: bool = False):
        bs, t, _ = x.shape

        query_key_value = self.qkv(x)
        query, key, value = query_key_value.chunk(3, dim=-1)
        query = self._split_heads(query)
        key = self._split_heads(key)
        value = self._split_heads(value)

        if layer_past is not None:
            past_key, past_value = layer_past
            key = torch.cat((past_key, key), dim=-2)
            value = torch.cat((past_value, value), dim=-2)

        if use_cache is True:
            present = (key, value)
        else:
            present = None

        attn_output = self.attention(query, key, value, attention_mask=attention_mask)
        attn_output = self._merge_heads(attn_output)
        attn_output = self.out(attn_output)
        outputs = (attn_output, present)
        return outputs
