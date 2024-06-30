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
                "starcoder2/15b/vllm",
                "starcoder2/3b",
                "starcoder2/7b",
                "starcoder2/15b"
            ]
        },
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {
                    "context_format": "default",
                    "rag_ratio": 0
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
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "gpt-3.5-turbo-1106",
                "gpt-3.5-turbo-0125",
                "gpt-4",
                "gpt-4-turbo",
                "gpt-4-turbo-2024-04-09"
            ]
        },
        "gpt-4o": {
            "n_ctx": 128000,
            "supports_tools": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
                "gpt-4o-2024-05-13"
            ]
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
            ]
        },
        "claude-3-5-sonnet": {
            "n_ctx": 16384,
            "supports_tools": true,
            "supports_scratchpads": {
                "PASSTHROUGH": {}
            },
            "similar_models": [
                "claude-3-5-sonnet-20240620"
            ]
        },
        "llama3/8b/instruct": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "<|start_header_id|>system<|end_header_id|>\n\n",
                    "keyword_user": "<|start_header_id|>user<|end_header_id|>\n\n",
                    "keyword_assistant": "<|start_header_id|>assistant<|end_header_id|>\n\n",
                    "eot": "<|eot_id|>",
                    "stop_list": [
                        "<|eot_id|>"
                    ],
                    "default_system_message": "You are a programming assistant."
                }
            },
            "similar_models": [
                "llama3/8b/instruct/vllm"
            ]
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
        "Refact/1.6B": "smallcloudai/Refact-1_6B-fim",
        "starcoder2/3b": "bigcode/starcoder2-3b",
        "text-embedding-3-small": "Xenova/text-embedding-ada-002",
        "gpt-3.5-turbo":          "Xenova/gpt-3.5-turbo-16k",
        "gpt-3.5-turbo-1106":     "Xenova/gpt-3.5-turbo-16k",
        "gpt-3.5-turbo-0125":     "Xenova/gpt-3.5-turbo-16k",
        "gpt-4":                  "Xenova/gpt-4",
        "gpt-4-turbo":            "Xenova/gpt-4",
        "gpt-4-turbo-2024-04-09": "Xenova/gpt-4",
        "gpt-4o":                 "Xenova/gpt-4o",
        "gpt-4o-2024-05-13":      "Xenova/gpt-4o",
        "claude-3-5-sonnet":          "Xenova/claude-tokenizer",
        "claude-3-5-sonnet-20240620": "Xenova/claude-tokenizer"
    }
}
"####;
