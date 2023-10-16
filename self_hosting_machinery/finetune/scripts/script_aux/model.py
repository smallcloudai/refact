import importlib
import logging
from functools import partial
from pathlib import Path
from typing import Dict, Any, List, Tuple

import deepspeed
import torch
from torchinfo import summary
from transformers import AutoTokenizer, AutoModelForCausalLM
from refact_models.lora import LoraMixin

from self_hosting_machinery.finetune.configuration import supported_models
from self_hosting_machinery.finetune.modelling.loss import masked_loss
from self_hosting_machinery.finetune.modelling.model_handling import map_model_specific_params
from self_hosting_machinery.finetune.utils import traces
from self_hosting_machinery.finetune.utils.timer import Timer

__all__ = ['ModelContext']


def _lora_state_dict(model, *args, destination=None, prefix='', keep_vars=False, layer_names):
    return {
        name: p
        for name, p in model.old_state_dict(
            *args, destination=destination, prefix=prefix, keep_vars=keep_vars
        ).items()
        if any(n in name for n in layer_names)
    }


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

        if debug:
            logging.info("1 gpumem_p0 %0.2fG" % (torch.cuda.max_memory_allocated() / 1e9))
            summary(self.model, depth=4, col_names=['num_params', 'params_percent', 'trainable'])

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
        freeze_exceptions, lora_target_modules = self._map_model_specific_params(
            freeze_exceptions, lora_target_modules
        )
        self._apply_model_modifiers(
            model
        )
        LoraMixin.apply_lora(
            model.to(device),
            lora_target_modules=lora_target_modules,
            lora_r=int(lora_r),
            lora_alpha=lora_alpha,
            lora_dropout=lora_dropout,
            lora_init_scale=lora_init_scale
        )
        self._freeze_model(
            model,
            freeze_exceptions=freeze_exceptions
        )
        model.old_state_dict = model.state_dict
        model.state_dict = partial(
            _lora_state_dict.__get__(model, type(model)),
            layer_names=freeze_exceptions
        )
        return model

    def forward(
            self,
            input: torch.Tensor
    ) -> torch.Tensor:
        logits = self.model.forward(
            input,
            return_dict=False,
            output_attentions=False,
            output_hidden_states=False
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
        keys_white_list = {
            'module', 'buffer_names', 'optimizer', 'param_shapes', 'frozen_param_shapes',
            'lr_scheduler', 'data_sampler', 'random_ltd', 'sparse_tensor_module_names',
            'skipped_steps', 'global_steps', 'global_samples', 'dp_world_size', 'mp_world_size',
            'ds_config', 'ds_version'
        }

        self.model.save_checkpoint(save_path, tag=tag)
        cp_path = Path(save_path) / tag
        model_cps = [p for p in cp_path.iterdir() if 'model_states' in p.name]
        _ = [p.unlink() for p in cp_path.iterdir() if 'model_states' not in p.name]
        for cp_path in model_cps:
            cp = torch.load(str(cp_path), map_location='cpu')
            cp = {k: v for k, v in cp.items() if k in keys_white_list}
            torch.save(cp, str(cp_path))

    def _freeze_model(
            self,
            model: torch.nn.Module,
            freeze_exceptions: List[str]
    ):
        for name, p in model.named_parameters():
            if any([e in name for e in freeze_exceptions]):
                p.requires_grad_(True)
            else:
                p.requires_grad_(False)

    def _setup_encoding(
            self,
            weights_path: str,
            repo_id: str
    ) -> AutoTokenizer:
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
            mod(model)

    def _set_low_gpu_mode(
            self,
            low_gpu_mode: bool
    ):
        self.low_gpu_mem_mode = low_gpu_mode

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
