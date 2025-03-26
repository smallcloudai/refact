pub const KNOWN_MODELS: &str = r####"
{
    "completion_models": {
        "bigcode/starcoder": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "context_format": "starcoder",
                    "rag_ratio": 0.5
                }
            },
            "default_scratchpad": "FIM-PSM",
            "tokenizer": "hf://bigcode/starcoder",
            "similar_models": [
                "bigcode/starcoderbase",
                "starcoder/15b/base",
                "starcoder/15b/plus",
                "starcoder/1b/base",
                "starcoder/3b/base",
                "starcoder/7b/base",
                "wizardcoder/15b",
                "starcoder/1b/vllm",
                "starcoder/3b/vllm",
                "starcoder/7b/vllm",
                "starcoder2/3b/base",
                "starcoder2/7b/base",
                "starcoder2/15b/base",
                "starcoder2/3b/vllm",
                "starcoder2/7b/vllm",
                "starcoder2/15b/vllm",
                "starcoder2/3b/neuron",
                "starcoder2/7b/neuron",
                "starcoder2/15b/neuron",
                "starcoder2/3b",
                "starcoder2/7b",
                "starcoder2/15b",
                "bigcode/starcoder2-3b",
                "bigcode/starcoder2-7b",
                "bigcode/starcoder2-15b"
            ]
        },
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-SPM": { }
            },
            "tokenizer": "hf://smallcloudai/Refact-1_6B-fim",
            "default_scratchpad": "FIM-SPM",
            "similar_models": [
                "Refact/1.6B",
                "Refact/1.6B/vllm"
            ]
        },
        "codellama/CodeLlama-13b-hf": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "fim_prefix": "<PRE>",
                    "fim_suffix": "<SUF>",
                    "fim_middle": "<MID>",
                    "eot": "<EOT>",
                    "eos": "</s>"
                }
            },
            "default_scratchpad": "FIM-PSM",
            "tokenizer": "hf://codellama/CodeLlama-13b-hf",
            "similar_models": [
                "codellama/7b"
            ]
        },
        "deepseek-coder/1.3b/base": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "fim_prefix": "<｜fim▁begin｜>",
                    "fim_suffix": "<｜fim▁hole｜>",
                    "fim_middle": "<｜fim▁end｜>",
                    "eot": "<|EOT|>"
                }
            },
            "default_scratchpad": "FIM-PSM",
            "tokenizer": "hf://deepseek-ai/deepseek-coder-1.3b-base",
            "similar_models": [
                "deepseek-coder/5.7b/mqa-base",
                "deepseek-coder/1.3b/vllm",
                "deepseek-coder/5.7b/vllm"
            ]
        },
        "stable/3b/code": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {}
            },
            "default_scratchpad": "FIM-PSM",
            "tokenizer": "hf://stabilityai/stable-code-3b",
            "similar_models": []
        },
        "llama3/8b/instruct": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "REPLACE": {
                    "token_bos": "<|begin_of_text|>",
                    "token_esc": "<|eot_id|>",
                    "keyword_system": "<|start_header_id|>system<|end_header_id|>\n\n",
                    "keyword_user": "<|start_header_id|>user<|end_header_id|>\n\n",
                    "keyword_assistant": "<|start_header_id|>assistant<|end_header_id|>\n\n",
                    "eot": "<|eot_id|>",
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "default_scratchpad": "REPLACE",
            "tokenizer": "hf://Xenova/llama3-tokenizer",
            "similar_models": [
                "llama3/8b/instruct/neuron",
                "llama3.1/8b/instruct",
                "llama3.2/3b/instruct",
                "llama3.2/1b/instruct"
            ]
        },
        "deepseek-coder/6.7b/instruct-finetune/vllm": {
            "n_ctx": 4096,
            "tokenizer": "hf://deepseek-ai/deepseek-coder-6.7b-instruct",
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            }
        },
        "llama3/8b/instruct/vllm": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/llama3-tokenizer",
            "similar_models": [
                "llama3.1/8b/instruct/vllm"
            ]
        },
        "llama3.2/1b/instruct/vllm": {
            "n_ctx": 16384,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://meta-llama/llama-3.2-1b-instruct",
            "similar_models": [
                "llama3.2/3b/instruct/vllm"
            ]
        },
        "qwen2.5/coder/1.5b/instruct/vllm": {
            "n_ctx": 32768,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Qwen/Qwen2.5-Coder-1.5B-Instruct",
            "similar_models": [
                "qwen2.5/coder/3b/instruct/vllm",
                "qwen2.5/coder/7b/instruct/vllm",
                "qwen2.5/coder/14b/instruct/vllm",
                "qwen2.5/coder/32b/instruct/vllm",
                "qwen2.5/7b/instruct/vllm",
                "qwen2.5/14b/instruct/vllm",
                "qwen2.5/32b/instruct/vllm"
            ]
        },
        "gpt-4o": {
            "n_ctx": 128000,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/gpt-4o",
            "similar_models": [
                "gpt-4o-2024-05-13",
                "gpt-4o-2024-08-06",
                "openai/gpt-4o",
                "gpt-4o-mini",
                "gpt-4o-mini-2024-07-18"
            ]
        },
        "claude-3-sonnet": {
            "n_ctx": 200000,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/claude-tokenizer",
            "similar_models": [
                "claude-3-haiku",
                "claude-3-5-haiku",
                "claude-3-5-haiku-20241022",
                "claude-3-opus",
                "claude-3-5-sonnet",
                "claude-3-5-sonnet-20241022",
                "claude-3-7-sonnet",
                "claude-3-7-sonnet-20250219"
            ]
        },
        "groq-llama-3.1-8b": {
            "n_ctx": 128000,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/Meta-Llama-3.1-Tokenizer",
            "similar_models": [
                "groq-llama-3.1-70b",
                "groq-llama-3.2-1b",
                "groq-llama-3.2-3b",
                "groq-llama-3.2-11b-vision",
                "groq-llama-3.2-90b-vision"
            ]
        },
        "cerebras-llama3.1-8b": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/Meta-Llama-3.1-Tokenizer",
            "similar_models": [
                "cerebras-llama3.1-70b"
            ]
        },
        "gemini-2.0-flash-exp": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/gemma2-tokenizer",
            "similar_models": [
                "gemini-1.5-flash",
                "gemini-1.5-flash-8b"
            ]
        },
        "gemini-1.5-pro": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/gemma2-tokenizer",
            "similar_models": [
                "gemini-2.0-exp-advanced"
            ]
        },
        "grok-beta": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer",
            "similar_models": [
                "grok-2-1212",
                "grok-2"
            ]
        },
        "grok-vision-beta": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "grok-2-vision-1212": {
            "n_ctx": 32000,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer",
            "similar_models": [
                "grok-2-vision"
            ]
        },
        "deepseek-chat": {
            "n_ctx": 64000,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://deepseek-ai/DeepSeek-V3"
        },
        "qwen2.5/coder/0.5b/instruct": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "REPLACE": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "<|im_start|>system\n",
                    "keyword_user": "<|im_start|>user\n",
                    "keyword_assistant": "<|im_start|>assistant\n",
                    "eot": "<|im_end|>",
                    "context_format": "chat",
                    "rag_ratio": 0.5
                }
            },
            "default_scratchpad": "REPLACE",
            "tokenizer": "hf://Qwen/Qwen2.5-Coder-0.5B-Instruct",
            "similar_models": [
                "qwen2.5/coder/1.5b/instruct",
                "qwen2.5/coder/3b/instruct",
                "qwen2.5/coder/7b/instruct/gptq8bit",
                "qwen2.5/coder/7b/instruct",
                "qwen2.5/coder/14b/instruct/gptq8bit",
                "qwen2.5/coder/14b/instruct",
                "qwen2.5/coder/32b/instruct/gptq8bit",
                "qwen2.5/coder/32b/instruct"
            ]
        },
        "qwen2.5/coder/0.5b/base": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "fim_prefix": "<|fim_prefix|>",
                    "fim_suffix": "<|fim_suffix|>",
                    "fim_middle": "<|fim_middle|>",
                    "eot": "<|endoftext|>",
                    "extra_stop_tokens": ["<|repo_name|>", "<|file_sep|>", "<|fim_pad|>", "<|cursor|>"],
                    "context_format": "qwen2.5",
                    "rag_ratio": 0.5
                }
            },
            "tokenizer": "hf://Qwen/Qwen2.5-Coder-0.5B",
            "default_scratchpad": "FIM-PSM",
            "similar_models": [
                "qwen2.5/coder/1.5b/base",
                "qwen2.5/coder/3b/base",
                "qwen2.5/coder/7b/base",
                "qwen2.5/coder/14b/base",
                "qwen2.5/coder/32b/base",
                "qwen2.5/coder/0.5b/base/vllm",
                "qwen2.5/coder/1.5b/base/vllm",
                "qwen2.5/coder/3b/base/vllm",
                "qwen2.5/coder/7b/base/vllm",
                "qwen2.5/coder/14b/base/vllm",
                "qwen2.5/coder/32b/base/vllm"
            ]
        }
    },
    "chat_models": {
        "gpt-4o": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                }
            },
            "tokenizer": "hf://Xenova/gpt-4o",
            "similar_models": [
                "gpt-4o-2024-05-13",
                "gpt-4o-2024-08-06",
                "openai/gpt-4o"
            ]
        },
        "gpt-4o-mini": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                }
            },
            "similar_models": [
                "gpt-4o-mini-2024-07-18"
            ],
            "tokenizer": "hf://Xenova/gpt-4o"
        },
        "o1": {
            "n_ctx": 200000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_reasoning": "openai",
            "supports_boost_reasoning": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                }
            },
            "tokenizer": "hf://Xenova/gpt-4o"
        },
        "o1-mini": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_reasoning": "openai",
            "supports_scratchpads": {
                "PASSTHROUGH": {
                }
            },
            "tokenizer": "hf://Xenova/gpt-4o"
        },
        "o3-mini": {
            "n_ctx": 200000,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_agent": true,
            "supports_reasoning": "openai",
            "supports_boost_reasoning": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                }
            },
            "tokenizer": "hf://Xenova/gpt-4o"
        },
        "claude-instant-1.2": {
            "n_ctx": 8096,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "claude-2.1",
                "claude-3-haiku",
                "claude-3-opus",
                "claude-3-sonnet"
            ],
            "tokenizer": "hf://Xenova/claude-tokenizer"
        },
        "claude-3-5-sonnet": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/claude-tokenizer",
            "similar_models": [
                "claude-3-5-sonnet-20240620"
            ]
        },
        "claude-3-5-sonnet-20241022": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_clicks": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/claude-tokenizer"
        },
        "claude-3-5-haiku": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_agent": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "claude-3-5-haiku-20241022"
            ],
            "tokenizer": "hf://Xenova/claude-tokenizer"
        },
        "claude-3-7-sonnet": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_clicks": true,
            "supports_agent": true,
            "supports_reasoning": "anthropic",
            "supports_boost_reasoning": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "claude-3-7-sonnet-20250219"
            ],
            "tokenizer": "hf://Xenova/claude-tokenizer"
        },
        "gemini-2.0-flash-exp": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "gemini-1.5-flash",
                "gemini-1.5-flash-8b"
            ],
            "tokenizer": "hf://Xenova/gemma2-tokenizer"
        },
        "gemini-1.5-pro": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "gemini-2.0-exp-advanced"
            ],
            "tokenizer": "hf://Xenova/gemma2-tokenizer"
        },
        "llama3/8b/instruct": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "<|start_header_id|>system<|end_header_id|>\n\n",
                    "keyword_user": "<|start_header_id|>user<|end_header_id|>\n\n",
                    "keyword_assistant": "<|start_header_id|>assistant<|end_header_id|>\n\n",
                    "eot": "<|eot_id|>",
                    "stop_list": [
                        "<|eot_id|>"
                    ]
                }
            },
            "tokenizer": "hf://Xenova/llama3-tokenizer",
            "similar_models": [
                "llama3/8b/instruct/neuron",
                "meta-llama/llama-3.1-8b-instruct",
                "llama3.1/8b/instruct",
                "llama3.2/3b/instruct",
                "llama3.2/1b/instruct"
            ]
        },
        "deepseek-coder/6.7b/instruct-finetune/vllm": {
            "n_ctx": 4096,
            "tokenizer": "hf://deepseek-ai/deepseek-coder-6.7b-instruct",
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            }
        },
        "llama3/8b/instruct/vllm": {
            "n_ctx": 8192,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://meta-llama/Meta-Llama-3-8B-Instruct",
            "similar_models": [
                "llama3.1/8b/instruct/vllm"
            ]
        },
        "llama3.2/1b/instruct/vllm": {
            "n_ctx": 16384,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://meta-llama/Llama-3.2-1B-Instruct",
            "similar_models": [
                "llama3.2/3b/instruct/vllm",
                "llama3.3/70b/instruct/vllm"
            ]
        },
        "mistral/24b/instruct/vllm": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "mistralai/Mistral-Small-24B-Instruct-2501",
            "similar_models": [
            ]
        },
        "qwen2.5/coder/1.5b/instruct/vllm": {
            "n_ctx": 32768,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Qwen/Qwen2.5-Coder-1.5B-Instruct",
            "similar_models": [
                "qwen2.5/coder/3b/instruct/vllm",
                "qwen2.5/coder/7b/instruct/vllm",
                "qwen2.5/coder/14b/instruct/vllm",
                "qwen2.5/coder/32b/instruct/vllm"
            ]
        },
        "qwen2.5/7b/instruct/vllm": {
            "n_ctx": 32768,
            "supports_tools": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Qwen/Qwen2.5-7B-Instruct",
            "similar_models": [
                "qwen2.5/14b/instruct/vllm",
                "qwen2.5/32b/instruct/vllm"
            ]
        },
        "qwen-qwq/32b/vllm": {
            "n_ctx": 32768,
            "supports_tools": true,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Qwen/QwQ-32B",
            "similar_models": [
                "qwen-qwq/32b/awq/vllm"
            ]
        },
        "wizardlm/7b": {
            "n_ctx": 2048,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "<s>",
                    "keyword_user": "\nUSER: ",
                    "keyword_assistant": "\nASSISTANT: ",
                    "eot": "",
                    "stop_list": ["\n\n"]
                }
            },
            "tokenizer": "hf://cognitivecomputations/WizardLM-7B-Uncensored",
            "similar_models": [
                "wizardlm/13b",
                "wizardlm/30b"
            ]
        },
        "magicoder/6.7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "\n@@ Instruction\n",
                    "keyword_assistant": "\n@@ Response\n",
                    "stop_list": [],
                    "eot": "<|EOT|>"
                }
            },
            "tokenizer": "hf://ise-uiuc/Magicoder-S-DS-6.7B"
        },
        "mistral/7b/instruct-v0.1": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "[INST] ",
                    "keyword_assistant": "[/INST]\n",
                    "stop_list": [],
                    "eot": "</s>"
                }
            },
            "tokenizer": "hf://mistralai/Mistral-7B-Instruct-v0.1",
            "similar_models": [
                "mixtral/8x7b/instruct-v0.1"
            ]
        },
        "phind/34b/v2": {
            "n_ctx": 4095,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "### System Prompt\n",
                    "keyword_user": "\n### User Message\n",
                    "keyword_assistant": "\n### Assistant\n",
                    "stop_list": [],
                    "eot": "</s>"
                }
            },
            "tokenizer": "hf://Phind/Phind-CodeLlama-34B-v2"
        },
        "deepseek-coder/6.7b/instruct": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_bos": "",
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "### Instruction:\n",
                    "keyword_assistant": "### Response:\n",
                    "stop_list": [],
                    "eot": "<|EOT|>"
                }
            },
            "tokenizer": "hf://deepseek-ai/deepseek-coder-6.7b-instruct",
            "similar_models": [
                "deepseek-coder/33b/instruct",
                "deepseek-coder/6.7b/instruct-finetune"
            ]
        },
        "groq-llama-3.1-8b": {
            "n_ctx": 128000,
            "supports_tools": false,
            "supports_multimodality": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "groq-llama-3.1-70b",
                "groq-llama-3.2-1b",
                "groq-llama-3.2-3b",
                "groq-llama-3.2-11b-vision",
                "groq-llama-3.2-90b-vision"
            ]
        },
        "cerebras-llama3.1-8b": {
            "n_ctx": 8192,
            "supports_tools": false,
            "supports_multimodality": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/Meta-Llama-3.1-Tokenizer",
            "similar_models": [
                "cerebras-llama3.1-70b"
            ]
        },
        "grok-beta": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "grok-vision-beta": {
            "n_ctx": 8192,
            "supports_tools": false,
            "supports_multimodality": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "grok-2-vision-1212": {
            "n_ctx": 32000,
            "supports_tools": true,
            "supports_multimodality": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "grok-2-1212": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "grok-2": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://Xenova/grok-1-tokenizer"
        },
        "deepseek-chat": {
            "n_ctx": 64000,
            "supports_tools": true,
            "supports_multimodality": false,
            "supports_agent": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://deepseek-ai/DeepSeek-V3"
        },
        "deepseek-reasoner": {
            "n_ctx": 64000,
            "supports_tools": false,
            "supports_multimodality": false,
            "supports_reasoning": "deepseek",
            "default_temperature": 0.6,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://deepseek-ai/DeepSeek-R1"
        },
        "qwen2.5/coder/0.5b/instruct": {
            "n_ctx": 8192,
            "supports_tools": false,
            "supports_multimodality": false,
            "supports_scratchpads": {
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
            },
            "tokenizer": "hf://Qwen/Qwen2.5-Coder-0.5B-Instruct",
            "similar_models": [
                "qwen2.5/coder/1.5b/instruct",
                "qwen2.5/coder/3b/instruct",
                "qwen2.5/coder/7b/instruct/gptq8bit",
                "qwen2.5/coder/7b/instruct",
                "qwen2.5/coder/14b/instruct/gptq8bit",
                "qwen2.5/coder/14b/instruct",
                "qwen2.5/coder/32b/instruct/gptq8bit",
                "qwen2.5/coder/32b/instruct"
            ]
        },
        "deepseek-r1-distill/1.5b/vllm": {
            "n_ctx": 32768,
            "supports_reasoning": "deepseek",
            "default_temperature": 0.6,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "tokenizer": "hf://deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B",
            "similar_models": [
                "deepseek-r1-distill/7b/vllm",
                "deepseek-r1-distill/8b/vllm",
                "deepseek-r1-distill/14b/vllm",
                "deepseek-r1-distill/32b/vllm",
                "deepseek-r1-distill/70b/vllm"
            ]
        }
    },
    "embedding_models": {
        "thenlper/gte-base": {
            "n_ctx": 512,
            "embedding_size": 768,
            "rejection_threshold": 0.25,
            "tokenizer": "hf://thenlper/gte-base"
        },
        "text-embedding-3-small": {
            "n_ctx": 8191,
            "embedding_size": 1536,
            "rejection_threshold": 0.63,
            "tokenizer": "hf://Xenova/text-embedding-ada-002"
        }
    }
}
"####;

// gemini and gemma bear the same tokenizer
// according to https://medium.com/google-cloud/a-gemini-and-gemma-tokenizer-in-java-e18831ac9677
// downloadable tokenizer.json does not exist for gemini, the only precise way is to use web-requests


// XAI WARNING: tokenizer is non-precise as there's no publicly available tokenizer for these models
// XAI says that for exact same model different tokenizers could be used
// therefore, using tokenizer for grok-1 which may or may not provide proximate enough results
