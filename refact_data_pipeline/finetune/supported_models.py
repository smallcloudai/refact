config = {
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
