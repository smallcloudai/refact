pub const KNOWN_MODELS: &str = r####"
{
    "code_completion_models": {
        "bigcode/starcoder": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "context_format": "starcoder",
                    "rag_ratio": 0.5
                },
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-PSM",
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
                "starcoder2/15b/vllm"
            ]
        },
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {
                    "context_format": "default",
                    "rag_ratio": 0.5
                }
            },
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
            "similar_models": [
                "deepseek-coder/5.7b/mqa-base",
                "deepseek-coder/1.3b/vllm",
                "deepseek-coder/5.7b/vllm"
            ]
        },
        "stable/3b/code": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-PSM",
            "similar_models": []
        }
    },
    "code_chat_models": {
        "meta-llama/Llama-2-70b-chat-hf": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            }
        },
        "gpt-3.5-turbo": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
            ]
        },
        "gpt-4": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
            ]
        },
        "claude-instant-1.2": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
                "claude-2.1"
            ]
        },
        "starchat/15b/beta": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "<|system|>\n",
                    "keyword_user": "<|end|>\n<|user|>\n",
                    "keyword_assistant": "<|end|>\n<|assistant|>\n",
                    "stop_list": [
                        "<|system|>",
                        "<|user|>",
                        "<|assistant|>",
                        "<|end|>",
                        "<empty_output>"
                    ],
                    "default_system_message": "You are a programming assistant."
                }
            }
        },
        "llama2/7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            },
            "similar_models": [
                "llama2/13b"
            ]
        },
        "wizardlm/7b": {
            "n_ctx": 2048,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "<s>",
                    "keyword_user": "\nUSER: ",
                    "keyword_assistant": "\nASSISTANT: ",
                    "eot": "",
                    "stop_list": ["\n\n"],
                    "default_system_message": "You are a helpful AI assistant.\n"
                }
            },
            "similar_models": [
                "wizardlm/13b",
                "wizardlm/30b"
            ]
        },
        "magicoder/6.7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "\n@@ Instruction\n",
                    "keyword_assistant": "\n@@ Response\n",
                    "stop_list": [],
                    "default_system_message": "You are an exceptionally intelligent coding assistant that consistently delivers accurate and reliable responses to user instructions.",
                    "eot": "<|EOT|>"
                }
            }
        },
        "mistral/7b/instruct-v0.1": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "[INST] ",
                    "keyword_assistant": "[/INST]\n",
                    "stop_list": [],
                    "default_system_message": "",
                    "eot": "</s>"
                }
            },
            "similar_models": [
                "mixtral/8x7b/instruct-v0.1"
            ]
        },
        "phind/34b/v2": {
            "n_ctx": 4095,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "### System Prompt\n",
                    "keyword_user": "\n### User Message\n",
                    "keyword_assistant": "\n### Assistant\n",
                    "stop_list": [],
                    "default_system_message": "You are an intelligent programming assistant.",
                    "eot": "</s>"
                }
            }
        },
        "deepseek-coder/6.7b/instruct": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "### Instruction:\n",
                    "keyword_assistant": "### Response:\n",
                    "stop_list": [],
                    "default_system_message": "You are an AI programming assistant, utilizing the Deepseek Coder model, developed by Deepseek Company, and you only answer questions related to computer science. For politically sensitive questions, security and privacy issues, and other non-computer science questions, you will refuse to answer.",
                    "eot": "<|EOT|>"
                }
            },
            "similar_models": [
                "deepseek-coder/33b/instruct",
                "deepseek-coder/6.7b/instruct-finetune",
                "deepseek-coder/6.7b/instruct-finetune/vllm"
            ]
        }
    },
    "tokenizer_rewrite_path": {
        "gpt-3.5-turbo": "Xenova/gpt-3.5-turbo-16k",
        "gpt-3.5-turbo-1106": "Xenova/gpt-3.5-turbo-16k",
        "gpt-4": "Xenova/gpt-4"
    }
}
"####;
