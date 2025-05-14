import einops
import torch

from typing import Tuple, Optional

from self_hosting_machinery.finetune.modelling.utils import get_base_model
from self_hosting_machinery.finetune.utils import traces


def apply_flash_mha_to_codellama_model(model):
    try:
        from flash_attn import flash_attn_func
    except ImportError:
        return

    def _forward(
            self,
            hidden_states: torch.Tensor,
            position_embeddings: Optional[Tuple[torch.Tensor, torch.Tensor]],
            attention_mask: Optional[torch.Tensor],
            past_key_value: Optional[Tuple[torch.Tensor]] = None,
            cache_position: Optional[torch.LongTensor] = None,
            *args, **kwargs
    ):
        from transformers.models.llama.modeling_llama import apply_rotary_pos_emb

        q, k, v = self.q_proj(hidden_states), self.k_proj(hidden_states), self.v_proj(hidden_states)
        q = einops.rearrange(q, "b t (h d) -> b h t d", h=self.config.num_attention_heads)
        k = einops.rearrange(k, "b t (h d) -> b h t d", h=self.config.num_key_value_heads)
        v = einops.rearrange(v, "b t (h d) -> b t h d", h=self.config.num_key_value_heads)

        cos, sin = position_embeddings
        q, k = apply_rotary_pos_emb(q, k, cos, sin)

        q = einops.rearrange(q, "b h t d -> b t h d")
        k = einops.rearrange(k, "b h t d -> b t h d")

        attn_output = flash_attn_func(
            q, k, v, softmax_scale=self.head_dim ** -0.5, causal=True
        )

        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")
        attn_output = self.o_proj(attn_output)
        return attn_output, None

    if torch.cuda.get_device_capability() < (8, 0):
        model.force_low_gpu_mem_mode = True
        torch.backends.cuda.enable_mem_efficient_sdp(False)
        traces.log("Flash attention is not supported on gpus with cuda capability < 8")
        return

    traces.log("Applying flash attention to the model")
    model = get_base_model(model)
    for layer in model.base_model.layers:
        layer.self_attn.forward = _forward.__get__(layer.self_attn, type(layer.self_attn))
