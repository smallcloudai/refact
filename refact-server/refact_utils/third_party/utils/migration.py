import json
import litellm

from pathlib import Path
from typing import Dict

from refact_utils.scripts import env
from refact_utils.third_party.utils.configs import ThirdPartyApiConfig
from refact_utils.third_party.utils.configs import ModelConfig
from refact_utils.third_party.utils.configs import ModelCapabilities
from refact_utils.third_party.utils.configs import ProviderConfig


KEY_PROVIDER_MAPPING = {
    "openai_api_key": "openai",
    "anthropic_api_key": "anthropic",
    "groq_api_key": "groq",
    "cerebras_api_key": "cerebras",
    "gemini_api_key": "gemini",
    "xai_api_key": "xai",
    "deepseek_api_key": "deepseek",
}


TOKENIZER_MAPPING = {
    "Xenova/gpt-4o": "gpt-4o",
    "Xenova/claude-tokenizer": "claude",
    "Xenova/Meta-Llama-3.1-Tokenizer": "llama-3-1",
    "Xenova/gemma2-tokenizer": "gemma2",
    "Xenova/grok-1-tokenizer": "grok-1",
    "deepseek-ai/DeepSeek-V3": "deepseek-v3",
    "deepseek-ai/DeepSeek-R1": "deepseek-r1",
}


def _model_config_from_model_dict(
        provider_id: str,
        api_key: str,
        model_dict: Dict,
) -> ModelConfig:
    # {
    #     "backend": "litellm",
    #     "provider": "openai",
    #     "tokenizer_path": "Xenova/gpt-4o",
    #     "resolve_as": "gpt-4o",
    #     "T": 128_000,
    #     "T_out": 4096,
    #     "pp1000t_prompt": 5_000,
    #     "pp1000t_generated": 15_000,  # $15.00 / 1M tokens (2024 may)
    #     "filter_caps": ["chat", "tools", "completion"],
    # }

    assert model_dict["provider"] == provider_id
    model_id = model_dict["resolve_as"]
    tools = bool(litellm.supports_function_calling(model_id))
    multimodal = bool(litellm.supports_vision(model_id))
    reasoning = provider_id if "reasoning" in model_dict.get("filter_caps", []) else None
    return ModelConfig(
        model_id=model_dict["resolve_as"],
        provider_id=provider_id,
        api_base=None,
        api_key=api_key,
        n_ctx=model_dict["T"],
        max_tokens=model_dict["T_out"],
        capabilities=ModelCapabilities(
            tools=tools,
            multimodal=multimodal,
            agent=tools and "agent" in model_dict.get("filter_caps", []),
            clicks=multimodal and "clicks" in model_dict.get("filter_caps", []),
            completion="completion" in model_dict.get("filter_caps", []),
            reasoning=reasoning,
            boost_reasoning=reasoning in ["openai", "anthropic"],
        ),
        tokenizer_id=TOKENIZER_MAPPING.get(model_dict["tokenizer_path"]),
    )


def _populate_models_for_provider(provider_id: str, api_key: str) -> Dict[str, ModelConfig]:
    from refact_known_models.passthrough import passthrough_mini_db

    model_configs = {}
    for model_dict in passthrough_mini_db.values():
        try:
            config = _model_config_from_model_dict(
                provider_id,
                api_key,
                model_dict,
            )
            model_configs[config.model_id] = config
        except Exception:
            pass

    return model_configs


def migrate_third_party_config():
    # {
    #     "openai_api_key": "",
    #     "anthropic_api_key": "",
    #     "groq_api_key": "",
    #     "cerebras_api_key": "",
    #     "gemini_api_key": "",
    #     "xai_api_key": "",
    #     "deepseek_api_key": "",
    #     "huggingface_api_key": ""
    # }

    integrations_cfg = Path(env.CONFIG_INTEGRATIONS)
    integrations = json.loads(integrations_cfg.read_text())

    providers = {}
    models = {}
    for provider_key, api_key in integrations.items():
        if not api_key or provider_key not in KEY_PROVIDER_MAPPING:
            continue
        provider_id = KEY_PROVIDER_MAPPING[provider_key]
        providers[provider_id] = ProviderConfig(enabled=True)
        models.update(_populate_models_for_provider(provider_id, api_key))

    integrations_cfg.rename(Path(f"{integrations_cfg}.bak"))
    with integrations_cfg.open("w") as f:
        json.dump({
            k: v for k, v in integrations.items()
            if k not in KEY_PROVIDER_MAPPING
        }, f, indent=4)

    return ThirdPartyApiConfig(
        providers=providers,
        models=models,
    )