import importlib
from collections import deque
from functools import partial
from pathlib import Path

from transformers import AutoModelForCausalLM, AutoTokenizer

from refact_data_pipeline.finetune import supported_models
from refact_encoding import RefactEncoding

import torch as th
import torch.nn.functional as F

from refact_models.codify_model import CodifyModel
from refact_models.checkpoint_loader import load_config

from typing import List, Tuple, Optional

from refact_models.lora import LoraMixin

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
                    "modelthinks=%-10s" % token_str(largest_logit_n,
                                                    (mask[b, ti].item() and labels[b, ti].item() != largest_logit_n),
                                                    "red"),
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
            p.requires_grad_(True)
        else:
            p.requires_grad_(False)
    return model


def save_model_state(model, save_path, tag):
    keys_white_list = {
        'module', 'buffer_names', 'optimizer', 'param_shapes', 'frozen_param_shapes',
        'lr_scheduler', 'data_sampler', 'random_ltd', 'sparse_tensor_module_names',
        'skipped_steps', 'global_steps', 'global_samples', 'dp_world_size', 'mp_world_size',
        'ds_config', 'ds_version'
    }
    
    print("model.save_checkpoint")
    model.save_checkpoint(save_dir=save_path, tag=tag)
    cp_path = Path(save_path) / tag
    print("model_cps")
    model_cps = [p for p in cp_path.iterdir() if 'model_states' in p.name]
    _ = [p.unlink() for p in cp_path.iterdir() if 'model_states' not in p.name]
    print("iterate through model_cps")
    for cp_path in model_cps:
        print(cp_path)
        print("load")
        cp = th.load(str(cp_path), map_location='cpu')
        cp = {k: v for k, v in cp.items() if k in keys_white_list}
        print("save")
        th.save(cp, str(cp_path))


def setup_encoding(
        model_name: str,
        weights_path: str,
        repo_id: str
):
    model_config = supported_models.config[model_name]
    if "tokenizer" in model_config:
        encoding = AutoTokenizer.from_pretrained(
            repo_id, cache_dir=weights_path,
            trust_remote_code=True
        )
        encoding.encode_stochastic = lambda x, *args, **kwargs: (encoding.encode(x), None)
        encoding.decode_utf8 = lambda x, *args, **kwargs: encoding.decode(x)
    else:
        encoding = RefactEncoding(
            load_config(root_path=weights_path, repo_id=repo_id).enc_name
        )
    encoding.EOT = model_config["tokenizer"]["eot_idx"]
    encoding.DIAMOND = model_config["tokenizer"]["padding_idx"]
    encoding.PREFIX = model_config["tokenizer"]["fim_prefix"]
    encoding.INFILL = model_config["tokenizer"]["fim_middle"]
    encoding.SUFFIX = model_config["tokenizer"]["fim_suffix"]
    encoding.ESCAPE = model_config["tokenizer"]["escape"]
    return encoding


def model_forward(
        model: th.nn.Module,
        input: th.Tensor,
        low_gpu_mem_mode: bool,
        backend: str
) -> th.Tensor:
    if backend == "transformers":
        if low_gpu_mem_mode:
            model.gradient_checkpointing_enable()

            def make_inputs_require_grad(module, input, output):
                output.requires_grad_(True)

            model.get_input_embeddings().register_forward_hook(make_inputs_require_grad)
        else:
            model.gradient_checkpointing_disable()
        logits = model.forward(
            input,
            return_dict=False, output_attentions=False, output_hidden_states=False
        )[0]
    else:
        if low_gpu_mem_mode:
            logits = model.forward_train_cp(input)
        else:
            logits = model.lm_forward(model(input, attention_mask=None)[0])
    return logits


def _lora_state_dict(model, *args, destination=None, prefix='', keep_vars=False, layer_names):
    return {
        name: p
        for name, p in model.old_state_dict(
            *args, destination=destination, prefix=prefix, keep_vars=keep_vars
        ).items()
        if any(n in name for n in layer_names)
    }


def setup_model_specific_params(
        model_name: str,
        freeze_exceptions: List[str],
        lora_target_modules: List[str]
) -> Tuple[List[str], List[str]]:
    assert model_name in supported_models.config
    model_config = supported_models.config[model_name]
    freeze_exceptions = [model_config["freeze_exceptions_mapping"][e] for e in freeze_exceptions]
    print(f'lora_target_modules: {lora_target_modules}')
    lora_target_modules_mapping = [m for modules in lora_target_modules
                                   for m in model_config["lora_target_modules_mapping"][modules]]
    return list(set(freeze_exceptions)), list(set(lora_target_modules_mapping))


def _apply_model_modifiers(model: th.nn.Module, modifiers: List[str]):
    for modifier in modifiers:
        path, modifier_name = modifier.rsplit('.', maxsplit=1)
        mod_path = importlib.import_module(f"refact_data_pipeline.finetune.{path}")
        mod = getattr(mod_path, modifier_name)
        mod(model)


def make_model(
        model_name: str,
        weights_path: str,
        repo_id: str,
        backend: str,
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
    encoding = setup_encoding(model_name, weights_path, repo_id)
    freeze_exceptions, lora_target_modules = setup_model_specific_params(
        model_name, freeze_exceptions, lora_target_modules
    )
    if backend == "legacy":
        model = CodifyModel.from_pretrained(
            weights_path, device=init_device, repo_id=repo_id
        ).to(dtype)
        _apply_model_modifiers(model, supported_models.config[model_name]['train_model_modifiers'])
    elif backend == "transformers":
        model = AutoModelForCausalLM.from_pretrained(
            repo_id, cache_dir=weights_path,
            device_map=init_device, torch_dtype=dtype,
            trust_remote_code=True
        )
        model.encoding = encoding
        _apply_model_modifiers(model, supported_models.config[model_name]['train_model_modifiers'])
    else:
        raise ValueError("Unknown backend")

    LoraMixin.apply_lora(
        model.to(device),
        lora_target_modules=lora_target_modules,
        lora_r=int(lora_r),
        lora_alpha=lora_alpha,
        lora_dropout=lora_dropout,
        lora_init_scale=lora_init_scale
    )
    model = freeze_model(
        model,
        freeze_exceptions=freeze_exceptions
    )
    model.old_state_dict = model.state_dict
    model.state_dict = partial(
        _lora_state_dict.__get__(model, type(model)),
        layer_names=freeze_exceptions
    )
    model = model.to(dtype)
    model = model.cuda()
    return model
