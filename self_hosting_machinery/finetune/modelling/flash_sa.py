import functools
import logging
import math

import einops
import torch
from typing import Tuple, Optional


@functools.lru_cache(maxsize=2)
def _get_alibi_slopes(attn_heads: int, dev: str) -> torch.Tensor:
    """
    ## Get head-specific slope $m$ for each head
    * `n_heads` is the number of heads in the attention layer $n$
    The slope for first head is
    $$\frac{1}{2^{\frac{8}{n}}} = 2^{-\frac{8}{n}}$$
    The slopes for the rest of the heads are in a geometric series with a ratio same as above.
    For instance when the number of heads is $8$ the slopes are
    $$\frac{1}{2^1}, \frac{1}{2^2}, \dots, \frac{1}{2^8}$$
    """

    # Get the closest power of 2 to `n_heads`.
    # If `n_heads` is not a power of 2, then we first calculate slopes to the closest (smaller) power of 2,
    # and then add the remaining slopes.
    n = 2 ** math.floor(math.log2(attn_heads))
    # $2^{-\frac{8}{n}}$
    m_0 = 2.0 ** (-8.0 / n)
    # $2^{-1\frac{8}{n}}, 2^{-2 \frac{8}{n}}, 2^{-3 \frac{8}{n}}, \dots$
    m = torch.pow(m_0, torch.arange(1, 1 + n, device=dev))

    # If `n_heads` is not a power of 2, then we add the remaining slopes.
    # We calculate the remaining slopes for $n * 2$ (avoiding slopes added previously).
    # And pick the slopes upto `n_heads`.
    if n < attn_heads:
        # $2^{-\frac{8}{2n}}$
        m_hat_0 = 2.0 ** (-4.0 / n)
        # $2^{-1\frac{8}{2n}}, 2^{-3 \frac{8}{2n}}, 2^{-5 \frac{8}{2n}}, \dots$
        # Note that we take steps by $2$ to avoid slopes added previously.
        m_hat = torch.pow(m_hat_0, torch.arange(1, 1 + 2 * (attn_heads - n), 2, device=dev))
        # Concatenate the slopes with the remaining slopes.
        m = torch.cat([m, m_hat])
    return m

def _prerequisites_are_ok(model, try_triton_kernel: bool):
    try:
        from flash_attn import flash_attn_func
        return True
    except ImportError:
        logging.warning("Original flash attention is not installed, trying to use triton implementation...")
        from self_hosting_machinery.finetune.modelling.triton_flash_sa import (apply_flash_mha_to_refact_model
                                                                               as apply_triton_flash)
        if try_triton_kernel:
            apply_triton_flash(model)
        return False


def apply_flash_mha_to_refact_model(model):
    if not _prerequisites_are_ok(model, try_triton_kernel=True):
        return

    from flash_attn import flash_attn_func

    def _forward(
            self,
            x: torch.Tensor,
            layer_past: Optional[torch.Tensor] = None,
            attention_mask: Optional[torch.Tensor] = None,
            alibi: Optional[torch.Tensor] = None,
            use_cache: Optional[bool] = False,
            output_attentions: Optional[bool] = False,
            *args, **kwargs
    ):
        q = einops.rearrange(self.q(x), "b t (h d) -> b t h d", h=self.num_heads)
        kv = einops.rearrange(self.kv(x), "b t (h d) -> b t h d", h=2)
        k, v = kv.chunk(2, dim=2)

        slopes = _get_alibi_slopes(self.num_heads, dev=q.device)
        attn_output = flash_attn_func(
            q, k, v, softmax_scale=self.scale_factor, causal=True,
            alibi_slopes=slopes
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
    if not _prerequisites_are_ok(model, try_triton_kernel=False):
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
            *args, **kwargs
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
        model.force_low_gpu_mem_mode = True
        logging.warning("Flash attention is not supported on gpus with cuda capability < 8")
        return

    logging.warning("Applying flash attention to the model")
    for block in model.transformer.h:
        block.attn.forward = _forward.__get__(block.attn, type(block.attn))


def apply_flash_mha_to_codellama_model(model):
    if not _prerequisites_are_ok(model, try_triton_kernel=False):
        return

    from flash_attn import flash_attn_func

    def _forward(
            self,
            hidden_states: torch.Tensor,
            attention_mask: Optional[torch.Tensor] = None,
            position_ids: Optional[torch.LongTensor] = None,
            past_key_value: Optional[Tuple[torch.Tensor]] = None,
            output_attentions: bool = False,
            use_cache: bool = False,
            *args, **kwargs
    ):
        from transformers.models.llama.modeling_llama import apply_rotary_pos_emb

        q, k, v = self.q_proj(hidden_states), self.k_proj(hidden_states), self.v_proj(hidden_states)
        q = einops.rearrange(q, "b t (h d) -> b h t d", h=self.num_heads)
        k = einops.rearrange(k, "b t (h d) -> b h t d", h=self.num_key_value_heads)
        v = einops.rearrange(v, "b t (h d) -> b t h d", h=self.num_key_value_heads)

        cos, sin = self.rotary_emb(v, seq_len=k.shape[-2])
        q, k = apply_rotary_pos_emb(q, k, cos, sin, position_ids)

        q = einops.rearrange(q, "b h t d -> b t h d")
        k = einops.rearrange(k, "b h t d -> b t h d")

        attn_output = flash_attn_func(
            q, k, v, softmax_scale=self.head_dim ** -0.5, causal=True
        )

        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")
        attn_output = self.o_proj(attn_output)
        return attn_output, None, None

    if torch.cuda.get_device_capability() < (8, 0):
        model.force_low_gpu_mem_mode = True
        torch.backends.cuda.enable_mem_efficient_sdp(False)
        logging.warning("Flash attention is not supported on gpus with cuda capability < 8")
        return

    logging.warning("Applying flash attention to the model")
    for layer in model.base_model.layers:
        layer.self_attn.forward = _forward.__get__(layer.self_attn, type(layer.self_attn))
