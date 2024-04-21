# refer to https://docs.litellm.ai/docs/providers/

passthrough_mini_db = {
    # gpt-4-turbo-2024-04-09 is already available, but no support from litellm yet.
    "gpt-4-turbo": {
        "backend": "openai",
        "provider": "openai",
        "tokenizer_path": "BEE-spoke-data/cl100k_base-mlm",
        "resolve_as": "gpt-4-turbo-2024-04-09",
        "T": 128_000,
        "T_out": 4096,
        "filter_caps": ["chat", "vision"]
    },
    "gpt-4": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-4",
        "resolve_as": "gpt-4-0125-preview",
        "T": 128_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    # gpt-3.5-turbo-0125 is already available, but no support from litellm yet.
    "gpt-3.5-turbo": {
        "backend": "litellm",
        "provider": "openai",
        "tokenizer_path": "Xenova/gpt-3.5-turbo-16k",
        "resolve_as": "gpt-3.5-turbo-1106",
        "T": 16_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    "claude-3-haiku": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-haiku-20240307",
        "T": 200_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    "claude-3-opus": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-opus-20240229",
        "T": 200_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    "claude-3-sonnet": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-3-sonnet-20240229",
        "T": 200_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    "claude-2.1": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-2.1",
        "T": 100_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
    "claude-instant-1.2": {
        "backend": "litellm",
        "provider": "anthropic",
        "tokenizer_path": "Xenova/claude-tokenizer",
        "resolve_as": "claude-instant-1.2",
        "T": 100_000,
        "T_out": 4096,
        "filter_caps": ["chat"]
    },
}
