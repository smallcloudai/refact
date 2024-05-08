huggingface_mini_db = {
    "starcoder/15b/base": {
        "backend": "autogptq",
        "model_path": "TheBloke/starcoder-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 18000,
        "T": 4096,
        "filter_caps": ["completion"],
        "deprecated": True,
    },
    "starcoder/15b/plus": {
        "backend": "autogptq",
        "model_path": "TheBloke/starcoderplus-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 18000,
        "T": 4096,
        "filter_caps": ["completion"],
        "deprecated": True,
    },
    "starchat/15b/beta": {
        "backend": "autogptq",
        "model_path": "TheBloke/starchat-beta-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 18000,
        "T": 4096,
        "filter_caps": ["chat", "starchat"],
        "deprecated": True,
    },
    "starcoder/1b/base": {
        "backend": "transformers",
        "model_path": "smallcloudai/starcoderbase-1b",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 8192,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder/3b/base": {
        "backend": "transformers",
        "model_path": "smallcloudai/starcoderbase-3b",
        "model_class_kwargs": {},
        "required_memory_mb": 12000,
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder/7b/base": {
        "backend": "transformers",
        "model_path": "smallcloudai/starcoderbase-7b",
        "model_class_kwargs": {},
        "required_memory_mb": 20000,
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "wizardcoder/15b": {
        "backend": "autogptq",
        "model_path": "TheBloke/WizardCoder-15B-1.0-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 18000,
        "T": 4096,
        "filter_caps": ["completion"],
    },
    "wizardlm/7b": {
        "backend": "autogptq",
        "model_path": "TheBloke/WizardLM-7B-V1.0-Uncensored-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 2048,
        "filter_caps": ["chat", "wizardlm"],
    },
    "wizardlm/13b": {
        "backend": "autogptq",
        "model_path": "TheBloke/WizardLM-13B-V1.1-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 14000,
        "T": 2048,
        "filter_caps": ["chat", "wizardlm"],
    },
    "llama2/7b": {
        "backend": "autogptq",
        "model_path": "TheBloke/Llama-2-7b-Chat-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 2048,
        "filter_caps": ["chat", "llama2"],
    },
    "llama2/13b": {
        "backend": "autogptq",
        "model_path": "TheBloke/Llama-2-13B-chat-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 14000,
        "T": 2048,
        "filter_caps": ["chat", "llama2"],
    },
    "codellama/7b": {
        "backend": "transformers",
        "model_path": "TheBloke/CodeLlama-7B-fp16",
        "model_class_kwargs": {},
        "required_memory_mb": 14000,
        "T": 2048,
        "filter_caps": ["completion", "finetune"],
    },
    "wizardlm/30b": {
        "backend": "transformers",
        "model_path": "TheBloke/WizardLM-30B-fp16",
        "model_class_kwargs": {
            "load_in_4bit": True,
        },
        "T": 2048,
        "filter_caps": ["chat", "wizardlm"],
    },
    "deepseek-coder/1.3b/base": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-1.3b-base",
        "model_class_kwargs": {},
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "deepseek-coder/5.7b/mqa-base": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-5.7bmqa-base",
        "model_class_kwargs": {},
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "deepseek-coder/6.7b/base": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-6.7b-base",
        "hidden": True,  # we see some kind of problem with this model (nan's while loss calculation)
        "model_class_kwargs": {},
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "magicoder/6.7b": {
        "backend": "autogptq",
        "model_path": "TheBloke/Magicoder-S-DS-6.7B-GPTQ",
        "model_class_kwargs": {
            "inject_fused_attention": False,
        },
        "required_memory_mb": 8000,
        "T": 4096,  # in fact this model allows 16k context, but we have 4k context at max in hf inference
        "filter_caps": ["chat"],
    },
    "mistral/7b/instruct-v0.1": {
        "backend": "autogptq",
        "model_path": "TheBloke/Mistral-7B-Instruct-v0.1-GPTQ",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 4096,  # in fact this model allows 8k context, but we have 4k context at max in hf inference
        "filter_caps": ["chat"],
    },
    "mixtral/8x7b/instruct-v0.1": {
        "backend": "transformers",
        "model_path": "mistralai/Mixtral-8x7B-Instruct-v0.1",
        "model_class_kwargs": {
            "load_in_4bit": True,
        },
        "required_memory_mb": 35000,
        "T": 4096,  # in fact this model allows 8k context, but we have 4k context at max in hf inference
        "filter_caps": ["chat"],
    },
    "phind/34b/v2": {
        "backend": "transformers",
        "model_path": "Phind/Phind-CodeLlama-34B-v2",
        "hidden": True,  # this model unexpectedly stops while sampling with </s> looks like a bug
        "model_class_kwargs": {
            "load_in_4bit": True,
        },
        "required_memory_mb": 24000,
        "T": 4095,
        "filter_caps": ["chat"],
    },
    "deepseek-coder/6.7b/instruct": {
        "backend": "autogptq",
        "model_path": "TheBloke/deepseek-coder-6.7B-instruct-GPTQ",
        "model_class_kwargs": {
            "inject_fused_attention": False,
        },
        "required_memory_mb": 8000,
        "T": 4096,  # in fact this model allows 16k context, but we have 4k context at max in hf inference
        "filter_caps": ["chat"],
    },
    "deepseek-coder/33b/instruct": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-33b-instruct",
        "model_class_kwargs": {
            "load_in_4bit": True,
        },
        "required_memory_mb": 24000,
        "T": 4096,  # in fact this model allows 16k context, but we have 4k context at max in hf inference
        "filter_caps": ["chat"],
    },
    "thenlper/gte-base": {
        "backend": "transformers",
        "model_path": "thenlper/gte-base",
        "model_class_kwargs": {},
        "T": 512,
        "filter_caps": ["embeddings"],
    },
    "stable/3b/code": {
        "backend": "transformers",
        "model_path": "stabilityai/stable-code-3b",
        "model_class_kwargs": {
            "attn_implementation": "flash_attention_2",
        },
        "required_memory_mb": 8000,
        "T": 4096,  # in fact this model allows 16k context, but we have 4k context at max in hf inference
        "filter_caps": ["completion"],
    },
    # NOTE: this model family uses tiktoken tokenizer, we cannot use them in refact-lsp for now
    # "stable/2b/lm": {
    #     "backend": "transformers",
    #     "model_path": "stabilityai/stablelm-2-1_6b",
    #     "model_class_kwargs": {
    #         "attn_implementation": "flash_attention_2",
    #     },
    #     "required_memory_mb": 8000,
    #     "T": 4096,
    #     "filter_caps": ["chat"],
    # },
    "starcoder2/3b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-3b",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder2/7b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-7b",
        "model_class_kwargs": {},
        "required_memory_mb": 16000,
        "T": 2048,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder2/15b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-15b",
        "model_class_kwargs": {},
        "required_memory_mb": 20000,
        "T": 4096,
        "filter_caps": ["completion", "finetune"],
    }
}
