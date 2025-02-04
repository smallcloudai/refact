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
    "thenlper/gte-base/cpu": {
        "backend": "transformers",
        "model_path": "thenlper/gte-base",
        "model_class_kwargs": {},
        "cpu": True,
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
        "filter_caps": ["completion", "chat"],
    },
    "llama3.1/8b/instruct": {
        "backend": "transformers",
        "model_path": "meta-llama/Llama-3.1-8B-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 20000,
        "T": 16384,  # in fact this model can handle 128K context
        "filter_caps": ["completion", "chat"],
    },
    "llama3.2/3b/instruct": {
        "backend": "transformers",
        "model_path": "meta-llama/Llama-3.2-3B-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 12000,
        "T": 16384,  # in fact this model can handle 128K context
        "filter_caps": ["completion", "chat"],
    },
    "llama3.2/1b/instruct": {
        "backend": "transformers",
        "model_path": "meta-llama/Llama-3.2-1B-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 8000,
        "T": 16384,  # in fact this model can handle 128K context
        "filter_caps": ["completion", "chat"],
    },
    # qwen 2.5-coder instruct models
    "qwen2.5/coder/32b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-32B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/14b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-14B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/7b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-7B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/3b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-3B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/1.5b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-1.5B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "chat"],
    },
    # qwen 2.5-coder completion models
    "qwen2.5/coder/32b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-32B",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/14b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-14B",
        "model_class_kwargs": {},
        "required_memory_mb": 35000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/7b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-7B",
        "model_class_kwargs": {},
        "required_memory_mb": 20000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/3b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-3B",
        "model_class_kwargs": {},
        "required_memory_mb": 15000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/1.5b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-1.5B",
        "model_class_kwargs": {},
        "required_memory_mb": 10000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/0.5b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-0.5B",
        "model_class_kwargs": {},
        "required_memory_mb": 7000,
        "T": 32768,
        "filter_caps": ["completion", "finetune"],
    },
}
