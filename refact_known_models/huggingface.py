huggingface_mini_db = {
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
    },
    # NOTE: we should add quantized model due to bnb quantization issues:
    # "load_in_8bit": True  works too slow
    # "load_in_4bit": True  too often generates bad results
    "llama3/8b/instruct": {
        "backend": "transformers",
        "model_path": "meta-llama/Meta-Llama-3-8B-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 20000,
        "T": 8192,
        "filter_caps": ["chat"],
    },
    "deepseek-coder-v2/16b/instruct": {
        "backend": "transformers",
        "model_path": "deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 80000,
        "T": 16384,  # in fact this model can handle 128K context
        "filter_caps": ["completion", "chat"],
    },
}
