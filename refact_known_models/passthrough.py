# refer to https://docs.litellm.ai/docs/providers/

passthrough_mini_db = {
    "gpt-4o": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4o",
        "resolve_as": "gpt-4o",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 5_000,
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens (2024 may)
        "filter_caps": ["chat", "tools", "completion"],
    },
    "gpt-4-turbo": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4",
        "resolve_as": "gpt-4-turbo",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 10_000,
        "pp1000t_generated": 30_000,  # $30.00 / 1M tokens (2024 may)
        "filter_caps": ["chat", "tools", "completion"],
    },
    "gpt-3.5-turbo": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-3.5-turbo-16k",
        "resolve_as": "gpt-3.5-turbo-1106",
        "T": 16_000,
        "T_out": 4096,
        "pp1000t_prompt": 1000,
        "pp1000t_generated": 2000,
        "filter_caps": ["chat", "tools", "completion"],
    },
    "claude-3-5-sonnet": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-5-sonnet-20240620",
        "T": 200_000,
        "T_out": 4096,
        "pp1000t_prompt": 3_000,  # $3.00 / 1M tokens (2024 jun)
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens (2024 jun)
        "filter_caps": ["chat", "tools", "completion"],
    },
    "claude-3-haiku": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-haiku-20240307",
        "T": 200_000,
        "T_out": 4096,
        "pp1000t_prompt": 250,
        "pp1000t_generated": 1_250,
        "filter_caps": ["chat", "tools", "completion"],
    },
    "claude-3-opus": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-opus-20240229",
        "T": 200_000,
        "T_out": 4096,
        "pp1000t_prompt": 15_000,
        "pp1000t_generated": 75_000,
        "filter_caps": ["chat", "tools", "completion"],
    },
    "claude-3-sonnet": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-sonnet-20240229",
        "T": 200_000,
        "T_out": 4096,
        "pp1000t_prompt": 3_000,
        "pp1000t_generated": 15_000,
        "filter_caps": ["chat", "tools", "completion"],
    },
    "gpt-4o-2024-05-13": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4o",
        "resolve_as": "gpt-4o-2024-05-13",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 5_000,
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens
        "filter_caps": ["chat", "tools", "completion"],
    },
    "gpt-4o-2024-08-06": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4o",
        "resolve_as": "gpt-4o-2024-08-06",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 2_500,
        "pp1000t_generated": 10_000,  # $15.00 / 1M tokens
        "filter_caps": ["chat", "tools", "completion"]
    },
    "gpt-4o-mini": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4o",
        "resolve_as": "gpt-4o-mini-2024-07-18",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # $0.60 / 1M tokens
        "filter_caps": ["chat", "tools", "completion"],
    },
    "claude-3-5-sonnet-20241022": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-5-sonnet-20241022",
        "T": 200_000,
        "T_out": 4096,
        "pp1000t_prompt": 3_000,  # $3.00 / 1M tokens (2024 oct)
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens (2024 oct)
        "filter_caps": ["chat", "tools", "completion"],
    },
    "groq-llama-3.1-8b": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.1-8b-instant",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "groq-llama-3.1-70b": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.1-70b-versatile",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "groq-llama-3.2-1b": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.2-1b-preview",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "groq-llama-3.2-3b": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.2-3b-preview",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "groq-llama-3.2-11b-vision": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.2-11b-vision-preview",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "groq-llama-3.2-90b-vision": {
        "backend": "litellm",
        "provider": "groq",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "groq/llama-3.2-90b-vision-preview",
        "T": 128_000,
        "T_out": 8000,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "cerebras-llama3.1-8b": {
        "backend": "litellm",
        "provider": "cerebras",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "cerebras/llama3.1-8b",
        "T": 8192,
        "T_out": 4096,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    },
    "cerebras-llama3.1-70b": {
        "backend": "litellm",
        "provider": "cerebras",
        "tokenizer_path": "Xenova/Meta-Llama-3.1-Tokenizer",
        "resolve_as": "cerebras/llama3.1-70b",
        "T": 8192,
        "T_out": 4096,
        "pp1000t_prompt": 150,
        "pp1000t_generated": 600,  # TODO: don't know the price
        "filter_caps": ["chat", "completion"],
    }
}
