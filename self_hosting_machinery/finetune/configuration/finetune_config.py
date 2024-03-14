import math
import torch
import torch.distributed as dist

from refact_utils.scripts import env
from self_hosting_machinery.finetune.utils import traces

from typing import Any, Dict, List


def base_config(model_name: str, models_db: Dict[str, Any]):
    if model_name not in models_db:
        raise RuntimeError(f"Unknown model {model_name}, try to update repo")
    model_info = models_db[model_name]
    if "finetune" not in model_info.get("filter_caps", []):
        raise RuntimeError(f"Model {model_name} does not support finetune")
    return dict(
        model_name=model_name,
        model_info=dict(
            weight_path=env.DIR_WEIGHTS,
            repo_id=model_info['model_path'],
            backend=model_info['backend'],
            ctx_size=model_info['T'],
            lora={
                "lora_target_modules": [
                    "qkv",
                    "backproj",
                    "mlp"
                ],
                "lora_r": 64,
                "lora_alpha": 128,
                "lora_dropout": 0.01,
            },
            freeze_exceptions=[
                "lora"
            ],
            loss_average_elements=16
        ),
        debug=False,
        limit_time_seconds=48 * 60 * 60,
        low_gpu_mem_mode=False,
        save_every=10,
        test_every=1,
        train_iters=5,
        scheduler={
            "type": "WarmupDecayLR",
            "params": {
                "total_num_steps": 250,
                "warmup_min_lr": 0,
                "warmup_max_lr": 30e-5,
                "warmup_num_steps": 20
            }
        },
        gradient_clipping=0.5,
        steps_per_print=int(1e9),
        train_batch_size=128,
        micro_batch_size=1,
        fp16={
            "enabled": True,
            "auto_cast": False,
            "loss_scale": 0,
            "initial_scale_power": 16,
            "loss_scale_window": 1000,
            "hysteresis": 1,
            "consecutive_hysteresis": False,
            "min_loss_scale": 1
        },
        zero_optimization={
            "stage": 2,
            "contiguous_gradients": False,
            "reduce_bucket_size": 1e5,
            "sub_group_size": 1e5,
            "offload_optimizer": {
                "device": "cpu",
                "pin_memory": False
            }
        },
        optimizer={
            "type": "AdamW",
            "params": {
                "lr": 30e-5,
                "betas": (0.9, 0.95),
                "eps": 1e-6,
                "weight_decay": 0.1,
            }
        }
    )


class ConfigBuilder:
    def __init__(self, cfg: Dict[str, Any]):
        self.cfg = cfg

    def set_train_steps(self, steps: int) -> 'ConfigBuilder':
        self.cfg['train_iters'] = steps
        return self

    def set_warmup_steps(self, steps: int) -> 'ConfigBuilder':
        self.cfg['scheduler']['params']['warmup_num_steps'] = steps
        return self

    def set_batch_size(self, bs: int) -> 'ConfigBuilder':
        self.cfg['train_batch_size'] = bs
        self.cfg['gradient_accumulation_steps'] = bs // self.cfg["micro_batch_size"] // dist.get_world_size()
        return self

    def set_lr(self, lr: float) -> 'ConfigBuilder':
        self.cfg['scheduler']['params']['warmup_max_lr'] = lr
        self.cfg['optimizer']['params']['lr'] = lr
        return self

    def set_weight_decay(self, decay: float) -> 'ConfigBuilder':
        self.cfg['optimizer']['params']['weight_decay'] = decay
        return self

    def set_lr_decay_steps(self, steps: float) -> 'ConfigBuilder':
        self.cfg['scheduler']['params']['total_num_steps'] = steps
        return self

    def set_lora_target_modules(self, modules: List[str]) -> 'ConfigBuilder':
        self.cfg['model_info']['lora']['lora_target_modules'] = modules
        return self

    def set_lora_r(self, lora_r: float) -> 'ConfigBuilder':
        self.cfg['model_info']['lora']['lora_r'] = lora_r
        return self

    def set_lora_alpha(self, lora_alpha: float) -> 'ConfigBuilder':
        self.cfg['model_info']['lora']['lora_alpha'] = lora_alpha
        return self

    def set_lora_dropout(self, dropout: float) -> 'ConfigBuilder':
        self.cfg['model_info']['lora']['lora_dropout'] = dropout
        return self

    def set_freeze_exceptions(self, exceptions: List[str]) -> 'ConfigBuilder':
        self.cfg['model_info']['freeze_exceptions'] = exceptions
        return self

    def set_save_every(self, save_every: int) -> 'ConfigBuilder':
        self.cfg['save_every'] = save_every
        return self

    def set_limit_time_seconds(self, seconds: int) -> 'ConfigBuilder':
        self.cfg['limit_time_seconds'] = seconds
        return self

    def set_low_gpu_mem_mode(self, low_gpu_mode: bool) -> 'ConfigBuilder':
        self.cfg['low_gpu_mem_mode'] = low_gpu_mode
        if self.cfg['low_gpu_mem_mode']:
            self.cfg['zero_optimization']['offload_optimizer'] = {
                "device": "cpu",
                "pin_memory": False
            }
        # TODO: check if free gpu memory >= 24GB
        # else:
        #     self.cfg['zero_optimization'].pop('offload_optimizer', None)
        return self

    def set_lora_quality_by_heuristics(
            self,
            initial_loss: float,
            ds_len: int
    ) -> 'ConfigBuilder':
        loss2score = {
            (0.0, 0.7): 0,
            (0.7, 0.8): 1,
            (0.8, 1.1): 2,
            (1.1, 1.5): 3,
            (1.5, 2.5): 4,
            (2.5, 100.0): 5
        }
        dslen2score = {
            (0, 500): 0,
            (500, 1000): 1,
            (1000, 2500): 2,
            (2500, 5000): 3,
            (5000, 10000): 5,
            (10000, 100000000): 8
        }

        complexity2config = {
            (0, 8): dict(lora_target_modules=[
                "qkv", "out", "mlp",
            ], lora_r=64, lora_alpha=128, lora_dropout=0.01,
                freeze_exceptions=[
                    "lora"
                ]),
            (8, 1000): dict(lora_target_modules=[
                "qkv", "out", "mlp",
            ], lora_r=64, lora_alpha=128, lora_dropout=0.01,
                freeze_exceptions=[
                    "wte", "lm_head", "lora"
                ])
        }

        complexity_score = 0
        for (lhs_loss, rhs_loss), score in loss2score.items():
            if lhs_loss <= initial_loss < rhs_loss:
                complexity_score += score
                break
        for (lhs_dslen, rhs_dslen), score in dslen2score.items():
            if lhs_dslen <= ds_len < rhs_dslen:
                complexity_score += score
                break

        for (lhs_score, rhs_score), lora_cfg in complexity2config.items():
            if lhs_score <= complexity_score < rhs_score:
                self.cfg['model_info']['freeze_exceptions'] = lora_cfg.pop('freeze_exceptions')
                self.cfg['model_info']['lora'] = lora_cfg
                break

        traces.log(
            f'Lora parameters heuristic avg_loss={initial_loss:.2f}, '
            f'ds_len={ds_len} => complexity score={complexity_score}'
        )

        return self

    def set_schedule_by_heuristics(
            self,
            ds_len: int
    ) -> 'ConfigBuilder':
        min_iterations = 50
        round_to_iter = 50
        dslen_per_epochs = {
            (0, 500): 50,
            (500, 1000): 50,
            (1000, 5000): 35,
            (5000, 15000): 20,
            (15000, 30000): 10,
            (30000, 100000000): 3
        }
        epochs = 1
        for (lhs_dslen, rhs_dslen), e in dslen_per_epochs.items():
            if lhs_dslen <= ds_len < rhs_dslen:
                epochs = e
                break

        effective_iters = max(epochs * (ds_len / self.cfg['train_batch_size']), min_iterations)
        effective_iters = int(math.ceil(effective_iters / round_to_iter) * round_to_iter)
        self.cfg['scheduler']['params']['total_num_steps'] = self.cfg['train_iters'] = effective_iters

        traces.log(f'Selected the schedule by heuristics ds_len={ds_len}:\n'
                   f'Total training steps: {self.cfg["train_iters"]}\n')

        return self

    def set_low_gpu_mem_mode_by_heuristics(self) -> 'ConfigBuilder':
        gpu_mem = torch.cuda.get_device_properties('cuda').total_memory
        self.set_low_gpu_mem_mode(gpu_mem < 20_000_000_000)
        traces.log(f'heuristic says low_gpu_mem_mode={gpu_mem < 20_000_000_000} by looking at gpu memory size')
        return self

    def set_trainable_embeddings(self, trainable_embeddings: bool) -> 'ConfigBuilder':
        if not trainable_embeddings:
            self.cfg['model_info']['freeze_exceptions'] = [
                e
                for e in self.cfg['model_info']['freeze_exceptions']
                if e not in {"wte", "lm_head"}
            ]
        else:
            freeze_exceptions = self.cfg['model_info']['freeze_exceptions'] + ["wte", "lm_head"]
            freeze_exceptions = list(set(freeze_exceptions))
            self.cfg['model_info']['freeze_exceptions'] = freeze_exceptions
        return self
