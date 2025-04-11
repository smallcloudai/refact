starcoder_supports_scratchpads = {
    "FIM-PSM": {
        "context_format": "starcoder",
        "rag_ratio": 0.5,
    },
}

deepseek_coder_supports_scratchpads = {
    "FIM-PSM": {
        "fim_prefix": "<｜fim▁begin｜>",
        "fim_suffix": "<｜fim▁hole｜>",
        "fim_middle": "<｜fim▁end｜>",
        "eot": "<|EOT|>",
    },
}

llama_supports_scratchpads = {
    "REPLACE": {
        "token_bos": "<|begin_of_text|>",
        "token_esc": "<|eot_id|>",
        "keyword_system": "<|start_header_id|>system<|end_header_id|>\n\n",
        "keyword_user": "<|start_header_id|>user<|end_header_id|>\n\n",
        "keyword_assistant": "<|start_header_id|>assistant<|end_header_id|>\n\n",
        "eot": "<|eot_id|>",
        "context_format": "chat",
        "rag_ratio": 0.5,
    }
}

llama_chat_supports_scratchpads = {
    "CHAT-GENERIC": {
        "token_bos": "",
        "token_esc": "",
        "keyword_system": "<|start_header_id|>system<|end_header_id|>\n\n",
        "keyword_user": "<|start_header_id|>user<|end_header_id|>\n\n",
        "keyword_assistant": "<|start_header_id|>assistant<|end_header_id|>\n\n",
        "eot": "<|eot_id|>",
        "stop_list": [
            "<|eot_id|>"
        ],
    }
}

qwen_coder_instruct_supports_scratchpads = {
    "REPLACE": {
        "token_bos": "",
        "token_esc": "",
        "keyword_system": "<|im_start|>system\n",
        "keyword_user": "<|im_start|>user\n",
        "keyword_assistant": "<|im_start|>assistant\n",
        "eot": "<|im_end|>",
        "context_format": "chat",
        "rag_ratio": 0.5,
    },
}

qwen_coder_instruct_chat_supports_scratchpads = {
    "CHAT-GENERIC": {
        "token_bos": "",
        "token_esc": "",
        "keyword_system": "<|im_start|>system\n",
        "keyword_user": "<|im_start|>user\n",
        "keyword_assistant": "<|im_start|>assistant\n",
        "eot": "<|im_end|>",
        "stop_list": [
            "<|im_end|>"
        ]
    }
}

qwen_coder_supports_scratchpads = {
    "FIM-PSM": {
        "fim_prefix": "<|fim_prefix|>",
        "fim_suffix": "<|fim_suffix|>",
        "fim_middle": "<|fim_middle|>",
        "eot": "<|endoftext|>",
        "extra_stop_tokens": ["<|repo_name|>", "<|file_sep|>", "<|fim_pad|>"],
        "context_format": "qwen2.5",
        "rag_ratio": 0.5
    }
}

huggingface_mini_db = {
    # starcoder2
    "starcoder2/3b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-3b",
        "model_class_kwargs": {},
        "required_memory_mb": 8000,
        "T": 4096,
        "supports_scratchpads": {
            "completion": starcoder_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder2/7b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-7b",
        "model_class_kwargs": {},
        "required_memory_mb": 16000,
        "T": 2048,
        "supports_scratchpads": {
            "completion": starcoder_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    "starcoder2/15b/base": {
        "backend": "transformers",
        "model_path": "bigcode/starcoder2-15b",
        "model_class_kwargs": {},
        "required_memory_mb": 20000,
        "T": 4096,
        "supports_scratchpads": {
            "completion": starcoder_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    # deepseek-coder
    "deepseek-coder/1.3b/base": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-1.3b-base",
        "model_class_kwargs": {},
        "T": 4096,
        "supports_scratchpads": {
            "completion": deepseek_coder_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    "deepseek-coder/5.7b/mqa-base": {
        "backend": "transformers",
        "model_path": "deepseek-ai/deepseek-coder-5.7bmqa-base",
        "model_class_kwargs": {},
        "T": 4096,
        "supports_scratchpads": {
            "completion": deepseek_coder_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    # llama
    "llama3.1/8b/instruct": {
        "backend": "transformers",
        "model_path": "meta-llama/Llama-3.1-8B-Instruct",
        "model_class_kwargs": {
            "torch_dtype": "bf16",
        },
        "required_memory_mb": 20000,
        "T": 16384,  # in fact this model can handle 128K context
        "supports_scratchpads": {
            "completion": llama_supports_scratchpads,
            "chat": llama_chat_supports_scratchpads,
        },
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
        "supports_scratchpads": {
            "completion": llama_supports_scratchpads,
            "chat": llama_chat_supports_scratchpads,
        },
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
        "supports_scratchpads": {
            "completion": llama_supports_scratchpads,
            "chat": llama_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    # qwen 2.5-coder instruct models
    "qwen2.5/coder/1.5b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-1.5B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_instruct_supports_scratchpads,
            "chat": qwen_coder_instruct_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/3b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-3B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_instruct_supports_scratchpads,
            "chat": qwen_coder_instruct_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/7b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-7B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_instruct_supports_scratchpads,
            "chat": qwen_coder_instruct_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/14b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-14B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_instruct_supports_scratchpads,
            "chat": qwen_coder_instruct_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    "qwen2.5/coder/32b/instruct": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-32B-Instruct",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_instruct_supports_scratchpads,
            "chat": qwen_coder_instruct_chat_supports_scratchpads,
        },
        "filter_caps": ["completion", "chat"],
    },
    # qwen 2.5-coder completion models
    "qwen2.5/coder/0.5b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-0.5B",
        "model_class_kwargs": {},
        "required_memory_mb": 7000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/1.5b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-1.5B",
        "model_class_kwargs": {},
        "required_memory_mb": 10000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/3b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-3B",
        "model_class_kwargs": {},
        "required_memory_mb": 15000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/7b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-7B",
        "model_class_kwargs": {},
        "required_memory_mb": 20000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/14b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-14B",
        "model_class_kwargs": {},
        "required_memory_mb": 35000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    "qwen2.5/coder/32b/base": {
        "backend": "transformers",
        "model_path": "Qwen/Qwen2.5-Coder-32B",
        "model_class_kwargs": {},
        "required_memory_mb": 45000,
        "T": 32768,
        "supports_scratchpads": {
            "completion": qwen_coder_supports_scratchpads,
        },
        "filter_caps": ["completion", "finetune"],
    },
    # embeddings
    "thenlper/gte-base": {
        "backend": "transformers",
        "model_path": "thenlper/gte-base",
        "model_class_kwargs": {},
        "T": 512,
        "size": 768,
        "filter_caps": ["embeddings"],
    },
    "thenlper/gte-base/cpu": {
        "backend": "transformers",
        "model_path": "thenlper/gte-base",
        "model_class_kwargs": {},
        "cpu": True,
        "T": 512,
        "size": 768,
        "filter_caps": ["embeddings"],
    },
}
