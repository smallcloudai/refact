__all__ = ['config']

_fim_train_ds_pipeline = {
    "ds_opts": "n_ctx={n_ctx},debug=0,seed=42,shuffle_depth=256,"
               "fim_probability=0.9,fim_drop_residual=1,random_trim_context_prob=0.01",
    "ds_name": "RefactFIMCodeDataset"
}
_fim_test_ds_pipeline = {
    "ds_opts": "n_ctx={n_ctx},debug=0,seed=42,shuffle_depth=0,quit_on_epoch=1,"
               "fim_probability=0.9,fim_drop_residual=1,random_trim_context_prob=0.01,"
               "pack_single=1,pack_complete=0,pack_buffer_size=50",
    "ds_name": "RefactFIMCodeDataset"
}
_bigcode_tokenizer_mapping = {
    "eot_idx": 0,
    "padding_idx": 4,
    "fim_prefix": 1,
    "fim_middle": 2,
    "fim_suffix": 3,
    "escape": 14
}

_starcoder_base = {
    "lora_target_modules_mapping": {
        "qkv": ["attn.q_attn", "attn.c_attn"],
        "out": ["attn.c_proj"],
        "backproj": ["attn.c_proj"],
        "mlp": ["mlp.c_fc", "mlp.c_proj"],
    },
    "freeze_exceptions_mapping": {
        "wte": ["wte", "wpe"],
        "lm_head": ["lm_head"],
        "lora": ["lora"]
    },
    "tokenizer": _bigcode_tokenizer_mapping,
    "train_ds_pipeline": _fim_train_ds_pipeline,
    "test_ds_pipeline": _fim_test_ds_pipeline,
    "train_model_modifiers": [
        "flash_sa.apply_flash_mha_to_starcoder_model"
    ],
    "force_enable_checkpointing": False
}
_starcoder2_base = {
    "lora_target_modules_mapping": {
        "qkv": ["self_attn.q_proj", "self_attn.k_proj", "self_attn.v_proj"],
        "out": ["self_attn.o_proj"],
        "backproj": ["self_attn.o_proj"],
        "mlp": ["mlp.c_fc", "mlp.c_proj"],
    },
    "freeze_exceptions_mapping": {
        "wte": ["embed_tokens"],
        "lm_head": ["lm_head"],
        "lora": ["lora"]
    },
    "tokenizer": _bigcode_tokenizer_mapping,
    "train_ds_pipeline": _fim_train_ds_pipeline,
    "test_ds_pipeline": _fim_test_ds_pipeline,
    "train_model_modifiers": [],
    "force_enable_checkpointing": False
}
_deepseek_base = {
    "lora_target_modules_mapping": {
        "qkv": ["self_attn.q_proj", "self_attn.k_proj", "self_attn.v_proj"],
        "out": ["self_attn.o_proj"],
        "backproj": ["self_attn.o_proj"],
        "mlp": ["mlp.gate_proj", "mlp.up_proj", "mlp.down_proj"],
    },
    "freeze_exceptions_mapping": {
        "wte": ["embed_tokens"],
        "lm_head": ["lm_head"],
        "lora": ["lora"]
    },
    "tokenizer": {
        "eot_idx": 32021,  # `<|EOT|>`
        "padding_idx": 32018,  # `<pad>`
        "fim_prefix": 32016,  # `<｜fim▁begin｜>`
        "fim_middle": 32017,  # `<｜fim▁end｜>`
        "fim_suffix": 32015,  # `<｜fim▁hole｜>`
        "escape": 32013,  # using `<｜begin▁of▁sentence｜>` token for now
    },
    "train_ds_pipeline": {
        "ds_opts": f"{_fim_train_ds_pipeline['ds_opts']},spm_prob=0.0",
        "ds_name": _fim_train_ds_pipeline["ds_name"]
    },
    "test_ds_pipeline": _fim_test_ds_pipeline,
    "train_model_modifiers": [
        "flash_sa.apply_flash_mha_to_codellama_model"
    ],
    "force_enable_checkpointing": False
}

config = {
    "Refact/1.6B": {
        "lora_target_modules_mapping": {
            "qkv": ["attn.q", "attn.kv"],
            "out": ["attn.c_proj"],
            "backproj": ["attn.c_proj"],
            "mlp": ["mlp.gate_up_proj", "mlp.c_proj"],
        },
        "freeze_exceptions_mapping": {
            "wte": ["wte"],
            "lm_head": ["lm_head"],
            "lora": ["lora"]
        },
        "tokenizer": _bigcode_tokenizer_mapping,
        "train_ds_pipeline": _fim_train_ds_pipeline,
        "test_ds_pipeline": _fim_test_ds_pipeline,
        "train_model_modifiers": [
            "flash_sa.apply_flash_mha_to_refact_model"
        ],
        "force_enable_checkpointing": False
    },

    "starcoder/1b/base": _starcoder_base,

    "starcoder/3b/base": _starcoder_base,

    "starcoder/7b/base": {
        **_starcoder_base,
        "force_enable_checkpointing": True
    },

    "starcoder2/3b/base": _starcoder2_base,

    "starcoder2/7b/base": {
        **_starcoder2_base,
        "force_enable_checkpointing": True
    },

    "starcoder2/15b/base": {
        **_starcoder2_base,
        "force_enable_checkpointing": True
    },

    "codellama/7b": {
        "lora_target_modules_mapping": {
            "qkv": ["self_attn.q_proj", "self_attn.k_proj", "self_attn.v_proj"],
            "out": ["self_attn.o_proj"],
            "backproj": ["self_attn.o_proj"],
            "mlp": ["mlp.gate_proj", "mlp.up_proj", "mlp.down_proj"],
        },
        "freeze_exceptions_mapping": {
            "wte": ["embed_tokens"],
            "lm_head": ["lm_head"],
            "lora": ["lora"]
        },
        "tokenizer": {
            "eot_idx": 32010,
            "padding_idx": 2,  # there is no padding token, so instead using `eos` token as in `gpt2`
            "fim_prefix": 32007,
            "fim_middle": 32009,
            "fim_suffix": 32008,
            "escape": 0,  # using <unk> token
            "bos_idx": 1
        },
        "train_ds_pipeline": {
            **_fim_train_ds_pipeline,
            "ds_name": "CodeLLamaFIMDataset"
        },
        "test_ds_pipeline": {
            **_fim_test_ds_pipeline,
            "ds_name": "CodeLLamaFIMDataset"
        },
        "train_model_modifiers": [
            "flash_sa.apply_flash_mha_to_codellama_model"
        ],
        "force_enable_checkpointing": True
    },

    "deepseek-coder/1.3b/base": _deepseek_base,

    "deepseek-coder/5.7b/mqa-base":  {
        **_deepseek_base,
        "force_enable_checkpointing": True
    },

    "deepseek-coder/6.7b/base": {
        **_deepseek_base,
        "force_enable_checkpointing": True
    }
}
