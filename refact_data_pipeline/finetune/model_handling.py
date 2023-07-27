from collections import deque
from functools import partial
from pathlib import Path
from refact_encoding import RefactEncoding

import einops
import torch as th
import torch.nn.functional as F

from refact_models.codify_model import CodifyModel
from refact_data_pipeline.finetune.sa import flash_attn_func


from typing import List, Tuple, Optional


unmasked_avg_buf = None


def masked_loss(
        logits: th.Tensor,
        labels: th.Tensor,
        mask: Optional[th.Tensor] = None,
        *,
        average_elements: int,
        enc: RefactEncoding,
        debug_dump: Optional[List[str]] = None
) -> th.Tensor:
    def _average(one_d_tensor: th.Tensor) -> th.Tensor:
        global unmasked_avg_buf
        if unmasked_avg_buf is None:
            unmasked_avg_buf = deque(maxlen=average_elements)
        if th.is_grad_enabled():
            for x in one_d_tensor:
                unmasked_avg_buf.append(float(x))
            return sum(unmasked_avg_buf) / len(unmasked_avg_buf)
        else:
            return one_d_tensor.to(th.float32).mean().item()

    _mb, _T = labels.shape
    mb, T, U = logits.shape
    assert _T == T
    assert _mb == mb

    ce = F.cross_entropy(
        logits.reshape(mb * T, U),
        labels.reshape(mb * T),
        reduction="none"
    ).reshape(mb, T)
    avg_mask_sum = _average(mask.sum(dim=1))
    loss_ce = ((ce * mask).sum(dim=1) / avg_mask_sum).mean()

    if debug_dump is not None:
        import termcolor
        def token_str(x, cond, color):
            t = "\"" + enc.decode([x]).replace("\n", "\\n") + "\""
            if cond:
                return termcolor.colored(t, color)
            else:
                return t
        with th.no_grad():
            b = 0
            for ti in range(T):
                if b == -1:
                    continue
                if ti & 15 == 0:
                    debug_dump.append("-----")
                largest_logit_n = logits[b, ti].argmax().item()
                debug_dump.append(" ".join([
                    "%04i" % (ti,),
                    "ce=%5.2f" % ce[b, ti].item(),
                    "label=%-20s" % token_str(labels[b, ti].item(), mask[b, ti].item(), "green"),
                    "mask=%i" % mask[b, ti].item(),
                    "largest_logit=%05i" % largest_logit_n,
                    "modelthinks=%-10s" % token_str(largest_logit_n, (mask[b, ti].item() and labels[b, ti].item() != largest_logit_n), "red"),
                ]))
        debug_dump.append("-- (ce * mask).sum(dim=1) = %s" % (ce * mask).sum(dim=1))
        debug_dump.append("-- avg_mask_sum = %s" % avg_mask_sum)
        debug_dump.append("-- this example loss_ce = %5.3f" % loss_ce.item())

    return loss_ce


def freeze_model(
        model: th.nn.Module,
        freeze_exceptions: List[str]
) -> th.nn.Module:
    for name, p in model.named_parameters():
        if any([e in name for e in freeze_exceptions]):
            continue
        p.requires_grad_(False)
    return model


def apply_flash_attention(model):
    def _forward(
            self,
            x: th.Tensor,
            attention_mask: Optional[th.Tensor],
            layer_past: Optional[Tuple[th.Tensor, th.Tensor]],
            use_cache: bool = False
    ):
        q, k, v = self.qkv(x).chunk(3, dim=-1)
        q = einops.rearrange(q, "b t (h d) -> b t h d", h=self.num_heads)
        k = einops.rearrange(k, "b t (h d) -> b t h d", h=self.num_heads)
        v = einops.rearrange(v, "b t (h d) -> b t h d", h=self.num_heads)

        attn_output = flash_attn_func(
            q, k, v, self.scale, True, True
        )
        attn_output = einops.rearrange(attn_output, "b t h d -> b t (h d)")

        attn_output = self.out(attn_output)
        return attn_output, None

    if type(model) != CodifyModel:
        raise NotImplementedError()
    else:
        for block in model.blocks:
            block.sa.forward = _forward.__get__(block.sa, type(block.sa))
        return model


def lora_state_dict(model, *args, destination=None, prefix='', keep_vars=False, layer_names):
    return {
        name: p
        for name, p in model.old_state_dict(
            *args, destination=destination, prefix=prefix, keep_vars=keep_vars
        ).items()
        if any(n in name for n in layer_names)
    }


def save_model_state(model, save_path, tag):
    keys_white_list = {
        'module', 'buffer_names', 'optimizer', 'param_shapes', 'frozen_param_shapes',
        'lr_scheduler', 'data_sampler', 'random_ltd', 'sparse_tensor_module_names',
        'skipped_steps', 'global_steps', 'global_samples', 'dp_world_size', 'mp_world_size',
        'ds_config', 'ds_version'
    }

    model.save_checkpoint(save_path, tag=tag)
    cp_path = Path(save_path) / tag
    model_cps = [p for p in cp_path.iterdir() if 'model_states' in p.name]
    _ = [p.unlink() for p in cp_path.iterdir() if 'model_states' not in p.name]
    for cp_path in model_cps:
        cp = th.load(str(cp_path), map_location='cpu')
        cp = {k: v for k, v in cp.items() if k in keys_white_list}
        th.save(cp, str(cp_path))


def make_model(
        weights_path: str,
        repo_id: str,
        *,
        freeze_exceptions: List[str],
        lora_target_modules: List[str],
        lora_r: int,
        lora_alpha: int,
        lora_dropout: float,
        lora_init_scale: float,
        dtype: th.dtype,
        init_device: str = "cpu",
        device: str = "cuda",
) -> th.nn.Module:
    # init_device CPU is to save memory
    model = CodifyModel.from_pretrained(
        weights_path, device=init_device, repo_id=repo_id
    ).to(dtype)
    model = model.apply_lora(
        model.to(device),
        lora_target_modules=lora_target_modules,
        lora_r=int(lora_r),
        lora_alpha=lora_alpha,
        lora_dropout=lora_dropout,
        lora_init_scale=lora_init_scale
    )
    if th.cuda.get_device_capability() >= (8, 0):
        model = apply_flash_attention(model)
    for param in list(model.parameters()):
        param.requires_grad = True
    model = freeze_model(
        model,
        freeze_exceptions=freeze_exceptions
    )
    model.old_state_dict = model.state_dict
    model.state_dict = partial(
        lora_state_dict.__get__(model, type(model)),
        layer_names=freeze_exceptions
    )
    model = model.to(dtype)
    model = model.cuda()
    return model
