import functools
import logging
import math

import einops
import torch
from typing import Tuple, Optional


@functools.lru_cache(maxsize=2)
def generate_alibi(
        max_seq_len: int,
        num_attention_heads: int,
        batch_size: Optional[int] = None,
        use_flash_attn: bool = True,
        tp_world_size: int = 1,
        tp_index: int = 0
) -> Tuple[torch.Tensor, float, float]:
    def get_slopes(n):
        def get_slopes_power_of_2(n):
            start = (2 ** (-2 ** -(math.log2(n) - 3)))
            ratio = start
            return [start * ratio ** i for i in range(n)]

        assert math.log2(n).is_integer(
        ), "it works only when num_attention_heads is power of 2"
        return get_slopes_power_of_2(n)

    slopes = torch.Tensor(get_slopes(num_attention_heads))
    alibi = slopes.unsqueeze(1).unsqueeze(1) * torch.arange(max_seq_len).unsqueeze(0).unsqueeze(0).expand(
        num_attention_heads, -1, -1)

    # Select the part of the tensor that corresponds to our tensor parallel index.
    alibi = alibi.reshape((tp_world_size, -1, *alibi.shape[1:]))[tp_index]

    if use_flash_attn:
        alibi = alibi.unsqueeze(0).contiguous()
        # (1, nheads, 1, seqlen_k)
    else:
        alibi = alibi.repeat(batch_size, 1, 1).contiguous()

    assert (num_attention_heads / tp_world_size).is_integer(
    ), "it works only when (num_attention_heads/tp_world_size) is integer"
    nh_tp = num_attention_heads // tp_world_size
    alibi_ratio = (2 ** (-2 ** -(math.log2(num_attention_heads) - 3)))
    alibi_start = (2 ** (-2 ** -(math.log2(num_attention_heads) - 3))) * alibi_ratio ** (nh_tp * tp_index)

    return alibi, alibi_start, alibi_ratio


def _prerequisites_are_ok(model):
    try:
        from flash_attn import flash_attn_func
        return True
    except ImportError:
        logging.warning("Original flash attention is not installed, trying to use triton implementation...")
        from self_hosting_machinery.finetune.modelling.triton_flash_sa import (apply_flash_mha_to_refact_model
                                                                               as apply_triton_flash)
        apply_triton_flash(model)
        return False


def apply_flash_mha_to_refact_model(model):
    if not _prerequisites_are_ok(model):
        return

    from flash_attn import flash_attn_func

    def _forward(
            self,
            x: torch.Tensor,
            layer_past: Optional[torch.Tensor] = None,
            attention_mask: Optional[torch.Tensor] = None,
            alibi: Optional[torch.Tensor] = None,
            use_cache: Optional[bool] = False,
            output_attentions: Optional[bool] = False
    ):
        q = einops.rearrange(self.q(x), "b t (h d) -> b t h d", h=self.num_heads)
        kv = einops.rearrange(self.kv(x), "b t (h d) -> b t h d", h=2)
        k, v = kv.chunk(2, dim=2)

        _, alibi_start, alibi_ratio = generate_alibi(q.shape[1], self.num_heads)
        attn_output = flash_attn_func(
            q, k, v, softmax_scale=self.scale_factor, causal=True,
            alibi=True, alibi_start=alibi_start, alibi_ratio=alibi_ratio
        )

        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")
        attn_output = self.c_proj(attn_output)
        return attn_output, None

    if torch.cuda.get_device_capability() < (8, 0):
        logging.warning("Triton flash attention is not supported on gpus with cuda capability < 8")
        return

    for block in model.transformer.h:
        block.attn.forward = _forward.__get__(block.attn, type(block.attn))


def apply_flash_mha_to_starcoder_model(model):
    if not _prerequisites_are_ok(model):
        return

    from flash_attn import flash_attn_func

    def _forward(
            self,
            x: torch.Tensor,
            layer_past: Optional[torch.Tensor] = None,
            attention_mask: Optional[torch.Tensor] = None,
            head_mask: Optional[torch.Tensor] = None,
            encoder_hidden_states: Optional[torch.Tensor] = None,
            encoder_attention_mask: Optional[torch.Tensor] = None,
            use_cache: Optional[bool] = False,
            output_attentions: Optional[bool] = False,
    ):
        qkv = self.c_attn(x)
        q = einops.rearrange(qkv[:, :, :self.embed_dim], "b t (h d) -> b t h d", h=self.num_heads)
        k = einops.rearrange(qkv[:, :, self.embed_dim:self.embed_dim + self.kv_dim], "b t (h d) -> b t h d", h=1)
        v = einops.rearrange(qkv[:, :, self.embed_dim + self.kv_dim:], "b t (h d) -> b t h d", h=1)

        scale_factor = self.head_dim ** -0.5
        attn_output = flash_attn_func(
            q, k, v, softmax_scale=scale_factor, causal=True,
        )

        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")
        attn_output = self.c_proj(attn_output)
        return attn_output, None

    if torch.cuda.get_device_capability() < (8, 0):
        return

    for block in model.transformer.h:
        block.attn.forward = _forward.__get__(block.attn, type(block.attn))
