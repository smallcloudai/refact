import math
import torch

from known_models_db.refact_known_models import models_mini_db
from refact_data_pipeline.finetune import traces
from self_hosting_machinery import env

from typing import Any, Dict, List


MODELS_CONFIGS = {
    "CONTRASTcode/3b/multi": {
        "lora_target_modules_mapping": {
            "qkv": ["qkv"],
            "out": ["out"],
            "backproj": ["out"],
            "mlp": ["mlp.ln_1", "mlp.ln_2"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 50256,
            "padding_idx": 48049,
            "fim_prefix": None,
            "fim_middle": None,
            "fim_suffix": None,
            "escape": 47171
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},pack_at_most=10,shuffle_depth=3000",
            "pipeline_name": "local_mix_plain_infill"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},pack_at_most=1,quit_on_epoch=1,seed=42",
            "pipeline_name": "local_sequence_plain_infill"
        },
        "train_model_modifiers": [
            "sa.apply_flash_mha_to_codify_model"
        ],
        "force_enable_checkpointing": False
    },

    "CONTRASTcode/medium/multi": {
        "lora_target_modules_mapping": {
            "qkv": ["qkv"],
            "out": ["out"],
            "backproj": ["out"],
            "mlp": ["mlp.ln_1", "mlp.ln_2"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 50256,
            "padding_idx": 48049,
            "fim_prefix": None,
            "fim_middle": None,
            "fim_suffix": None,
            "escape": 47171
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},pack_at_most=10,shuffle_depth=3000",
            "pipeline_name": "local_mix_plain_infill"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},pack_at_most=1,quit_on_epoch=1,seed=42",
            "pipeline_name": "local_sequence_plain_infill"
        },
        "train_model_modifiers": [
            "sa.apply_flash_mha_to_codify_model"
        ],
        "force_enable_checkpointing": False
    },

    "Refact/1.6B": {
        "lora_target_modules_mapping": {
            "qkv": ["attn.q", "attn.k", "attn.v"],
            "out": ["attn.c_proj"],
            "backproj": ["attn.c_proj"],
            "mlp": ["mlp.linear_1", "mlp.c_proj", "mlp.linear_3"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 0,
            "padding_idx": 4,
            "fim_prefix": 1,
            "fim_middle": 2,
            "fim_suffix": 3,
            "escape": 14
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42",
            "pipeline_name": "local_fim"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42,"
                       "pack_single=1,pack_complete=0,pack_buffer_size=25,"
                       "quit_on_epoch=1,seed=42",
            "pipeline_name": "local_fim"
        },
        "train_model_modifiers": [
            "sa.apply_flash_mha_to_refact_model"
        ],
        "force_enable_checkpointing": False
    },

    "starcoder/1b/base": {
        "lora_target_modules_mapping": {
            "qkv": ["attn.q_attn", "attn.c_attn"],
            "out": ["attn.c_proj"],
            "backproj": ["attn.c_proj"],
            "mlp": ["mlp.c_fc", "mlp.c_proj"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 0,
            "padding_idx": 4,
            "fim_prefix": 1,
            "fim_middle": 2,
            "fim_suffix": 3,
            "escape": 14
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42",
            "pipeline_name": "local_fim"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42,"
                       "pack_single=1,pack_complete=0,pack_buffer_size=25,"
                       "quit_on_epoch=1,seed=42",
            "pipeline_name": "local_fim"
        },
        "train_model_modifiers": [],
        "force_enable_checkpointing": True
    },

    "starcoder/3b/base": {
        "lora_target_modules_mapping": {
            "qkv": ["attn.q_attn", "attn.c_attn"],
            "out": ["attn.c_proj"],
            "backproj": ["attn.c_proj"],
            "mlp": ["mlp.c_fc", "mlp.c_proj"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 0,
            "padding_idx": 4,
            "fim_prefix": 1,
            "fim_middle": 2,
            "fim_suffix": 3,
            "escape": 14
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42,seed=42",
            "pipeline_name": "local_fim"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42,"
                       "pack_single=1,pack_complete=0,pack_buffer_size=25,"
                       "quit_on_epoch=1,seed=42",
            "pipeline_name": "local_fim"
        },
        "train_model_modifiers": [],
        "force_enable_checkpointing": True
    },

    "starcoder/7b/base": {
        "lora_target_modules_mapping": {
            "qkv": ["attn.q_attn", "attn.c_attn"],
            "out": ["attn.c_proj"],
            "backproj": ["attn.c_proj"],
            "mlp": ["mlp.c_fc", "mlp.c_proj"],
        },
        "freeze_exceptions_mapping": {
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": {
            "eot_idx": 0,
            "padding_idx": 4,
            "fim_prefix": 1,
            "fim_middle": 2,
            "fim_suffix": 3,
            "escape": 14
        },
        "train_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42",
            "pipeline_name": "local_fim"
        },
        "test_ds_pipeline": {
            "ds_opts": "n_ctx={n_ctx},fim_probability=0.9,fim_drop_residual=1,"
                       "tkr_stochastic_tokens=3,shuffle_depth=3000,debug=0,"
                       "random_trim_context_prob=0.01,fim_random_seed=42,"
                       "pack_single=1,pack_complete=0,pack_buffer_size=25,"
                       "quit_on_epoch=1,seed=42",
            "pipeline_name": "local_fim"
        },
        "train_model_modifiers": [],
        "force_enable_checkpointing": True
    }
}


def base_config(model_name: str):
    if model_name not in models_mini_db:
        raise RuntimeError(f"Unknown model {model_name}, try to update repo")
    model_info = models_mini_db[model_name]
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
                ],
                "lora_r": 16,
                "lora_alpha": 32,
                "lora_dropout": 0.01,
                "lora_init_scale": 0.01
            },
            freeze_exceptions=[
                "lora",
            ],
            loss_average_elements=16
        ),
        debug=False,
        limit_time_seconds=48 * 60 * 60,
        low_gpu_mem_mode=True,
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
        gradient_accumulation_steps=128,
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
        self.cfg['gradient_accumulation_steps'] = bs
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

    def set_lora_init_scale(self, init_scale: float) -> 'ConfigBuilder':
        self.cfg['model_info']['lora']['lora_init_scale'] = init_scale
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
        losses_per_scores = {
            (0.0, 0.9): 0,
            (0.9, 1.3): 1,
            (1.3, 1.7): 2,
            (1.7, 2.3): 3,
            (2.3, 2.6): 4,
            (2.6, 100.0): 5
        }
        dslen_per_scores = {
            (0, 500): 0,
            (500, 1000): 1,
            (1000, 5000): 2,
            (5000, 15000): 3,
            (15000, 30000): 5,
            (30000, 100000000): 8
        }

        scores_per_loraconfigs = {
            (0, 1000): dict(lora_target_modules=[
                "qkv", "out", "mlp",
            ], lora_r=64, lora_alpha=128, lora_dropout=0.01, lora_init_scale=0.01,
                freeze_exceptions=[
                    "wte", "lm_head", "lora"
                ]),
        }

        score_acc = 0
        for (lhs_loss, rhs_loss), score in losses_per_scores.items():
            if lhs_loss <= initial_loss < rhs_loss:
                score_acc += score
                break
        for (lhs_dslen, rhs_dslen), score in dslen_per_scores.items():
            if lhs_dslen <= ds_len < rhs_dslen:
                score_acc += score
                break

        for (lhs_score, rhs_score), lora_cfg in scores_per_loraconfigs.items():
            if lhs_score <= score_acc < rhs_score:
                self.cfg['model_info']['freeze_exceptions'] = lora_cfg.pop('freeze_exceptions')
                self.cfg['model_info']['lora'] = lora_cfg
                break

        traces.log(
            f'Lora parameters heuristic avg_loss={initial_loss:.2f}, '
            f'ds_len={ds_len} => complexity score={score_acc}'
        )

        return self

    def set_schedule_by_heuristics(
            self,
            ds_len: int
    ) -> 'ConfigBuilder':
        min_iterations = 50
        round_to_iter = 50
        dslen_per_epochs = {
            (0, 500): 60,
            (500, 1000): 60,
            (1000, 5000): 45,
            (5000, 15000): 25,
            (15000, 30000): 15,
            (30000, 100000000): 5
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
        traces.log(f'Selected low_gpu_mem_mode={gpu_mem < 20_000_000_000} by total gpu memory\n')
        return self
