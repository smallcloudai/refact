import json
import os
import litellm

from typing import Dict, List, Optional

from refact_utils.scripts import env
from refact_utils.third_party.utils.configs import ThirdPartyApiConfig
from refact_utils.third_party.utils.configs import ProviderConfig
from refact_utils.third_party.utils.configs import ModelConfig
from refact_utils.third_party.utils.configs import ModelCapabilities
from refact_utils.third_party.utils.migration import migrate_third_party_config


__all__ = [
    "load_third_party_config",
    "save_third_party_config",
    "available_third_party_models",
    "get_provider_models",
]


def _validate_config(config: ThirdPartyApiConfig, raise_on_error: bool):
    # Filter out models whose provider is not in the current configuration
    models = {}
    for model_id, model_config in config.models.items():
        if model_config.provider_id in config.providers:
            models[model_id] = model_config
        elif raise_on_error:
            raise RuntimeError(f"no provider for `{model_id}` model")
    config.models = models
    # Correct capabilities
    for model_config in config.models.values():
        if model_config.capabilities.agent and not model_config.capabilities.tools:
            if raise_on_error:
                raise RuntimeError(f"agent capability requires tools")
            model_config.capabilities.agent = False
        if model_config.capabilities.clicks and not model_config.capabilities.multimodal:
            if raise_on_error:
                raise RuntimeError(f"clicks capability requires multimodal")
            model_config.capabilities.clicks = False
    return config


def load_third_party_config() -> ThirdPartyApiConfig:
    if os.path.exists(env.CONFIG_INTEGRATIONS) and not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
        migrate_third_party_config()
    try:
        if not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            raise FileNotFoundError(f"No third party config found")
        with open(env.CONFIG_INTEGRATIONS_MODELS, "r") as f:
            data = json.load(f)
        config = ThirdPartyApiConfig.model_validate(data)
        return _validate_config(config, raise_on_error=False)
    except Exception:
        return ThirdPartyApiConfig()


def save_third_party_config(config: ThirdPartyApiConfig):
    os.makedirs(os.path.dirname(env.CONFIG_INTEGRATIONS_MODELS), exist_ok=True)
    config = _validate_config(config, raise_on_error=True)
    with open(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", "w") as f:
        json.dump(config.model_dump(), f, indent=4)
    os.rename(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", env.CONFIG_INTEGRATIONS_MODELS)


def available_third_party_models() -> Dict[str, ModelConfig]:
    config = load_third_party_config()
    models_available = {}

    def _is_enabled(provider_id: str, providers: Dict[str, ProviderConfig]) -> bool:
        if provider_id is None:
            return True  # custom model without provider_id
        if provider_id not in providers:
            return False  # should not happen, provider is not presented
        return providers[provider_id].enabled

    for model_name, model_config in config.models.items():
        if not _is_enabled(model_config.provider_id, config.providers):
            continue
        try:
            models_available[model_name] = model_config
        except Exception:
            pass

    return models_available


def _get_default_model_config(provider_id: str, model_id: str) -> ModelConfig:
    def _get_context_size(model_name: str) -> int:
        PASSTHROUGH_N_CTX_LIMIT = 128_000
        model_info = litellm.get_model_info(model_name)
        return min(model_info.get("max_input_tokens") or 8192, PASSTHROUGH_N_CTX_LIMIT)

    def _get_max_tokens(model_name: str) -> Optional[int]:
        PASSTHROUGH_MAX_TOKENS_LIMIT = 16_000
        return min(litellm.get_max_tokens(model_name) or 8192, PASSTHROUGH_MAX_TOKENS_LIMIT)

    return ModelConfig(
        model_id=model_id,
        provider_id=provider_id,
        api_base=None,
        api_key=None,
        n_ctx=_get_context_size(model_id),
        max_tokens=_get_max_tokens(model_id),
        capabilities=ModelCapabilities(
            tools=bool(litellm.supports_function_calling(model_id)),
            multimodal=bool(litellm.supports_vision(model_id)),
            agent=False,
            clicks=False,
            completion=False,
        ),
        tokenizer_id=None,
    )


def get_provider_models() -> Dict[str, List[str]]:
    providers_models = {
        "custom": [],
    }
    for provider in litellm.provider_list:
        provider_id = str(provider.value)
        for model_id in litellm.models_by_provider.get(provider_id, []):
            try:
                model_info = litellm.get_model_info(model=model_id)
                if model_info and model_info.get("mode") == "chat":
                    model_config = _get_default_model_config(provider_id, model_id)
                    providers_models.setdefault(provider_id, []).append(model_config.dict())
            except Exception:
                continue
    return providers_models
