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
    },

    "codellama/7b": {
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
            "eot_idx": 6,
            "padding_idx": 7,
            "fim_prefix": 3,
            "fim_middle": 4,
            "fim_suffix": 5,
            "escape": 7,
            "bos": 1,
            "eos": 2
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
