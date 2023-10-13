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
        "wte": "wte",
        "lm_head": "lm_head",
        "lora": "lora"
    },
    "tokenizer": _bigcode_tokenizer_mapping,
    "train_ds_pipeline": _fim_train_ds_pipeline,
    "test_ds_pipeline": _fim_test_ds_pipeline,
    "train_model_modifiers": [],
    "force_enable_checkpointing": True
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
            "wte": "wte",
            "lm_head": "lm_head",
            "lora": "lora"
        },
        "tokenizer": _bigcode_tokenizer_mapping,
        "train_ds_pipeline": _fim_train_ds_pipeline,
        "test_ds_pipeline": _fim_test_ds_pipeline,
        "train_model_modifiers": [
            "triton_flash_sa.apply_flash_mha_to_refact_model"
        ],
        "force_enable_checkpointing": False
    },

    "starcoder/1b/base": _starcoder_base,

    "starcoder/3b/base": _starcoder_base,

    "starcoder/7b/base": _starcoder_base
}
