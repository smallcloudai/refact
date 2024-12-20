# refer to https://docs.litellm.ai/docs/providers/

passthrough_mini_db = {
    # OpenAI models
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

    # Anthropic models
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

    # Groq models
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

    # Cerebras models
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
    },

    # gemini and gemma bear the same tokenizer
    # according to https://medium.com/google-cloud/a-gemini-and-gemma-tokenizer-in-java-e18831ac9677
    # downloadable tokenizer.json does not exist for gemini, proposed solution to use vertexai lib in python uses web requests
    # for pricing consult: https://ai.google.dev/pricing
    # pricing below is assumed for <= 128_000 context is used

    "gemini-2.0-flash-exp": {
        "backend": "litellm",
        "provider": "gemini",
        "tokenizer_path": "Xenova/gemma2-tokenizer",
        "resolve_as": "gemini/gemini-2.0-flash-exp",
        "T": 1_048_576,
        "T_out": 8_192,
        "pp1000t_prompt": 75,  # $0.075 / 1M tokens
        "pp1000t_generated": 300,  # $0.30 / 1M tokens
        "filter_caps": ["chat", "tools", "completion", "vision"],
    },
    "gemini-1.5-flash": {
        "backend": "litellm",
        "provider": "gemini",
        "tokenizer_path": "Xenova/gemma2-tokenizer",
        "resolve_as": "gemini/gemini-1.5-flash",
        "T": 1_048_576,
        "T_out": 8_192,
        "pp1000t_prompt": 75,  # $0.075 / 1M tokens
        "pp1000t_generated": 300,  # $0.30 / 1M tokens
        "filter_caps": ["chat", "tools", "completion", "vision"],
    },
    "gemini-1.5-flash-8b": {
        "backend": "litellm",
        "provider": "gemini",
        "tokenizer_path": "Xenova/gemma2-tokenizer",
        "resolve_as": "gemini/gemini-1.5-flash-8b",
        "T": 1_048_576,
        "T_out": 8_192,
        "pp1000t_prompt": 37.5,  # $0.0375 / 1M tokens
        "pp1000t_generated": 150,  # $0.15 / 1M tokens
        "filter_caps": ["chat", "tools", "completion", "vision"],
    },
    "gemini-1.5-pro": {
        "backend": "litellm",
        "provider": "gemini",
        "tokenizer_path": "Xenova/gemma2-tokenizer",
        "resolve_as": "gemini/gemini-1.5-pro",
        "T": 2_097_152,
        "T_out": 8_192,
        "pp1000t_prompt": 1250,  # $1.25 / 1M tokens
        "pp1000t_generated": 5000,  # $5.00 / 1M tokens
        "filter_caps": ["chat", "tools", "completion", "vision"],
    },
    # XAI Models
    # WARNING: tokenizer is non-precise as there's no publicly available tokenizer for these models
    # XAI says that for exact same model different tokenizers could be used
    # therefore, using tokenizer for grok-1 which may or may not provide proximate enough results
    # T is decreased not to encounter tokens overflow

    "grok-beta": {
        "backend": "litellm",
        "provider": "xai",
        "tokenizer_path": "Xenova/grok-1-tokenizer",
        "resolve_as": "xai/grok-beta",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 5_000,
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens
        "filter_caps": ["chat", "completion"],
    },
    "grok-vision-beta": {
        "backend": "litellm",
        "provider": "xai",
        "tokenizer_path": "Xenova/grok-1-tokenizer",
        "resolve_as": "xai/grok-vision-beta",
        "T": 8_192,
        "T_out": 4096,
        "pp1000t_prompt": 5_000,
        "pp1000t_generated": 15_000,  # $15.00 / 1M tokens
        "filter_caps": ["chat", "completion", "vision"],
    },
    "grok-2-vision-1212": {
        "backend": "litellm",
        "provider": "xai",
        "tokenizer_path": "Xenova/grok-1-tokenizer",
        "resolve_as": "xai/grok-2-vision-1212",
        "T": 32_000,
        "T_out": 4096,
        "pp1000t_prompt": 2_000,
        "pp1000t_generated": 10_000,  # $10.00 / 1M tokens
        "filter_caps": ["chat", "completion", "vision"],
    },
    "grok-2-1212": {
        "backend": "litellm",
        "provider": "xai",
        "tokenizer_path": "Xenova/grok-1-tokenizer",
        "resolve_as": "xai/grok-2-1212",
        "T": 128_000,
        "T_out": 4096,
        "pp1000t_prompt": 2_000,
        "pp1000t_generated": 10_000,  # $10.00 / 1M tokens
        "filter_caps": ["chat", "completion"],
    }
}
