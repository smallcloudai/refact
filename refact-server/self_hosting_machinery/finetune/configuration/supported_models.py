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

_qwen_base = {
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
        "eot_idx": 151643,  # `<|endoftext|>`
        "padding_idx": 151662,  # `<|fim_pad|>`
        "fim_prefix": 151659,  # `<|fim_prefix|>`
        "fim_middle": 151660,  # `<|fim_middle|>`
        "fim_suffix": 151661,  # `<|fim_suffix|>`
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
    # qwen models
    "qwen2.5/coder/32b/base": {
        **_qwen_base,
        "force_enable_checkpointing": True
    },
    "qwen2.5/coder/14b/base": {
        **_qwen_base,
        "force_enable_checkpointing": True
    },
    "qwen2.5/coder/7b/base": {
        **_qwen_base,
        "force_enable_checkpointing": True
    },
    "qwen2.5/coder/3b/base": {
        **_qwen_base,
        "force_enable_checkpointing": False
    },
    "qwen2.5/coder/1.5b/base": {
        **_qwen_base,
        "force_enable_checkpointing": False
    },
    "qwen2.5/coder/0.5b/base": {
        **_qwen_base,
        "force_enable_checkpointing": False
    }
}
