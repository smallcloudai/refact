import importlib
import json
import logging
import os
from collections import defaultdict
from functools import partial
from pathlib import Path
from typing import Dict, Any, List, Tuple

import deepspeed
import safetensors
import torch
from safetensors.torch import save_file
from torchinfo import summary
from transformers import AutoTokenizer, AutoModelForCausalLM
from peft import get_peft_config, get_peft_model, LoraConfig, TaskType

from self_hosting_machinery.finetune.configuration import supported_models
from self_hosting_machinery.finetune.modelling.lora import LoraMixin
from self_hosting_machinery.finetune.modelling.loss import masked_loss
from self_hosting_machinery.finetune.modelling.utils import map_model_specific_params
from self_hosting_machinery.finetune.utils import traces
from self_hosting_machinery.finetune.utils.timer import Timer

__all__ = ["ModelContext"]


def _lora_state_dict(model, *args, destination=None, prefix='', keep_vars=False, layer_names):
    return {
        name: p
        for name, p in model.old_state_dict(
            *args, destination=destination, prefix=prefix, keep_vars=keep_vars
        ).items()
        if any(n in name for n in layer_names)
    }


def _shared_pointers(tensors):
    ptrs = defaultdict(list)
    for k, v in tensors.items():
        ptrs[v.data_ptr()].append(k)
    failing = []
    for ptr, names in ptrs.items():
        if len(names) > 1:
            failing.append(names)
    return failing


class ModelContext:
    def __init__(
            self,
            finetune_cfg: Dict[str, Any],
            use_deepspeed: bool = False,
            debug: bool = False
    ):
        self.model_name = finetune_cfg["model_name"]
        self.finetune_cfg = finetune_cfg
        self.model_mappings_config = supported_models.config[self.model_name]
        self.low_gpu_mem_hook = None
        with Timer(message="/model load {time_ms:.1f}ms"):
            self.model = self._make_model(
                weights_path=self.finetune_cfg['model_info']['weight_path'],
                repo_id=self.finetune_cfg['model_info']['repo_id'],
                freeze_exceptions=self.finetune_cfg['model_info']['freeze_exceptions'],
                lora_target_modules=self.finetune_cfg['model_info']['lora']['lora_target_modules'],
                lora_r=self.finetune_cfg['model_info']['lora']['lora_r'],
                lora_alpha=self.finetune_cfg['model_info']['lora']['lora_alpha'],
                lora_dropout=self.finetune_cfg['model_info']['lora']['lora_dropout'],
                lora_init_scale=self.finetune_cfg['model_info']['lora']['lora_init_scale'],
                dtype=(torch.bfloat16 if 'bf16' in self.finetune_cfg and self.finetune_cfg['bf16']['enabled']
                       else torch.float16),
                init_device="cuda",
                device="cuda",
            )
            self._set_low_gpu_mode(
                self.finetune_cfg['low_gpu_mem_mode']
                or self.model_mappings_config['force_enable_checkpointing']
            )
        self.encoding = self.model.encoding

        if use_deepspeed:
            with Timer(message="/deepspeed initialization {time_ms:.1f}ms"):
                self.model, _, _, _ = deepspeed.initialize(
                    config=self.finetune_cfg,
                    model=self.model,
                    model_parameters=[p for p in self.model.parameters() if p.requires_grad],
                    dist_init_required=True
                )
                self.use_deepspeed = True

        logging.info("Allocated memory: %0.2fG" % (torch.cuda.max_memory_allocated() / 1e9))
        print(summary(self.model, depth=4, col_names=['num_params', 'params_percent', 'trainable']))

        self.loss_fn = partial(
            masked_loss,
            average_elements=self.finetune_cfg['model_info']['loss_average_elements'],
            enc=self.encoding
        )

    def _make_model(
            self,
            weights_path: str,
            repo_id: str,
            *,
            freeze_exceptions: List[str],
            lora_target_modules: List[str],
            lora_r: int,
            lora_alpha: int,
            lora_dropout: float,
            lora_init_scale: float,
            dtype: torch.dtype,
            init_device: str = "cpu",
            device: str = "cuda",
    ) -> torch.nn.Module:
        model = AutoModelForCausalLM.from_pretrained(
            repo_id,
            cache_dir=weights_path,
            device_map=init_device,
            torch_dtype=dtype,
            trust_remote_code=True
        )
        model.encoding = self._setup_encoding(
            weights_path=self.finetune_cfg['model_info']['weight_path'],
            repo_id=self.finetune_cfg['model_info']['repo_id']
        )
        if len(lora_target_modules) > 0:
            freeze_exceptions, lora_target_modules = self._map_model_specific_params(
                freeze_exceptions, lora_target_modules
            )
            model = get_peft_model(model, LoraConfig(
                target_modules=lora_target_modules,
                modules_to_save=freeze_exceptions,
                task_type=TaskType.CAUSAL_LM,
                inference_mode=False,
                r=int(lora_r),
                lora_alpha=lora_alpha,
                lora_dropout=lora_dropout,
            ))
        self._freeze_model(
            model,
            freeze_exceptions=freeze_exceptions
        )
        self._apply_model_modifiers(model)
        return model

    def forward(
            self,
            input: torch.Tensor
    ) -> torch.Tensor:
        logits = self.model.forward(
            input,
            return_dict=False,
            output_attentions=False,
            output_hidden_states=False,
            use_cache=False
        )[0]
        return logits

    def loss(
            self,
            logits: torch.Tensor,
            labels: torch.Tensor,
            mask: torch.Tensor
    ) -> torch.Tensor:
        loss = self.loss_fn(
            logits=logits,
            labels=labels,
            mask=mask,
        )
        return loss

    def backward(
            self, loss: torch.Tensor
    ):
        assert self.use_deepspeed
        try:
            self.model.backward(loss)
        except torch.cuda.OutOfMemoryError as e:
            if self.low_gpu_mem_mode:
                raise e
            else:
                self.model.optimizer.zero_grad()
                torch.cuda.empty_cache()
                self._set_low_gpu_mode(low_gpu_mode=True)
                traces.log("switching to low GPU memory mode")
                self.backward(loss)

    def step(self):
        assert self.use_deepspeed
        self.model.step()

    def train_information(self) -> Dict[str, Any]:
        if self.use_deepspeed:
            return dict(gpumem_p0=torch.cuda.max_memory_allocated())

        return dict(
            gpumem_p0=torch.cuda.max_memory_allocated(),
            lr=self.model.optimizer.param_groups[-1]['lr'],
            num_skipped_updates=self.model.skipped_steps,
            scale=self.model.optimizer.cur_scale,
        )

    def train(self):
        self.model.train()

    def eval(self):
        self.model.eval()

    def save_model_state(
            self,
            save_path: str,
            tag: str
    ):
        output_path = os.path.join(save_path, tag)
        self.model.save_pretrained(output_path, safe_serialization=True)
        weights = safetensors.torch.load_file(os.path.join(output_path, "adapter_model.safetensors"))
        lora_weights, embeddings_weights = {}, {}
        for key in weights.keys():
            if "lora" in key:
                lora_weights[key] = weights[key]
            else:
                embeddings_weights[key] = weights[key]
        safetensors.torch.save_file(lora_weights, os.path.join(output_path, "adapter_model.safetensors"))
        if len(embeddings_weights) > 0:
            safetensors.torch.save_file(embeddings_weights, os.path.join(output_path, "new_embeddings.safetensors"))

    def _freeze_model(
            self,
            model: torch.nn.Module,
            freeze_exceptions: List[str]
    ):
        for name, p in model.named_parameters(remove_duplicate=False):
            if any([e in name for e in freeze_exceptions]):
                p.requires_grad_(True)
            else:
                p.requires_grad_(False)

    def _setup_encoding(
            self,
            weights_path: str,
            repo_id: str
    ) -> AutoTokenizer:
        os.environ["TOKENIZERS_PARALLELISM"] = "false"

        assert "tokenizer" in self.model_mappings_config
        encoding = AutoTokenizer.from_pretrained(
            repo_id, cache_dir=weights_path,
            trust_remote_code=True
        )
        encoding.EOT = self.model_mappings_config["tokenizer"]["eot_idx"]
        encoding.DIAMOND = self.model_mappings_config["tokenizer"]["padding_idx"]
        encoding.PREFIX = self.model_mappings_config["tokenizer"]["fim_prefix"]
        encoding.INFILL = self.model_mappings_config["tokenizer"]["fim_middle"]
        encoding.SUFFIX = self.model_mappings_config["tokenizer"]["fim_suffix"]
        encoding.ESCAPE = self.model_mappings_config["tokenizer"]["escape"]
        encoding.BOS = self.model_mappings_config["tokenizer"].get("bos_idx", None)

        return encoding

    def _map_model_specific_params(
            self,
            freeze_exceptions: List[str],
            lora_target_modules: List[str]
    ) -> Tuple[List[str], List[str]]:
        return map_model_specific_params(
            model_name=self.model_name,
            freeze_exceptions=freeze_exceptions,
            lora_target_modules=lora_target_modules
        )

    def _apply_model_modifiers(
            self,
            model: torch.nn.Module
    ):
        for modifier in self.model_mappings_config['train_model_modifiers']:
            path, modifier_name = modifier.rsplit('.', maxsplit=1)
            mod_path = importlib.import_module(f"self_hosting_machinery.finetune.modelling.{path}")
            mod = getattr(mod_path, modifier_name)
            try:
                mod(model)
            except Exception as e:
                logging.error(f"Applying model modifier {mod_path} wasn't successful: {e}")

    def _set_low_gpu_mode(
            self,
            low_gpu_mode: bool
    ):
        force_low_gpu_mem_mode = hasattr(self.model, "force_low_gpu_mem_mode") and self.model.force_low_gpu_mem_mode
        self.low_gpu_mem_mode = low_gpu_mode or force_low_gpu_mem_mode
        logging.warning(f"Setting low_gpu_mem_mode={self.low_gpu_mem_mode} for the model")

        if self.low_gpu_mem_mode:
            self.model.gradient_checkpointing_enable()

            def make_inputs_require_grad(module, input, output):
                output.requires_grad_(True)

            self.low_gpu_mem_hook = self.model.get_input_embeddings().register_forward_hook(
                make_inputs_require_grad
            )
        else:
            self.model.gradient_checkpointing_disable()
            if self.low_gpu_mem_hook is not None:
                self.low_gpu_mem_hook.remove()
