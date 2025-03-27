import json
import os
import litellm

from pydantic import BaseModel, Field
from typing import Dict, List, Any, Optional

from refact_utils.scripts import env
from refact_utils.third_party.utils.migration import migrate_third_party_config


__all__ = [
    "load_third_party_config",
    "save_third_party_config",
    "available_third_party_models",
    "get_provider_models",
    "ThirdPartyApiConfig",
    "ProviderConfig",
    "ModelConfig",
    "ModelCapabilities",
]


class ModelCapabilities(BaseModel):
    tools: bool
    multimodal: bool
    agent: bool
    clicks: bool
    completion: bool


class ModelConfig(BaseModel):
    model_id: str
    provider_id: str
    api_base: Optional[str]
    api_key: Optional[str]
    n_ctx: int
    max_tokens: int
    capabilities: ModelCapabilities
    tokenizer_id: Optional[str] = None

    # TODO: validation of the config

    # NOTE: weird function for backward compatibility
    def compose_usage_dict(self, prompt_tokens_n: int, generated_tokens_n: int) -> Dict[str, int]:
        def _pp1000t(cost_entry_name: str) -> int:
            cost = litellm.model_cost.get(self.model_id, {}).get(cost_entry_name, 0)
            return int(cost * 1_000_000 * 1_000)
        return {
            "pp1000t_prompt": _pp1000t("input_cost_per_token"),
            "pp1000t_generated": _pp1000t("output_cost_per_token"),
            "metering_prompt_tokens_n": prompt_tokens_n,
            "metering_generated_tokens_n": generated_tokens_n,
        }

    def to_completion_model_record(self) -> Dict[str, Any]:
        assert self.capabilities.completion
        return {
            "n_ctx": self.n_ctx,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5,
                }
            },
        }

    def to_chat_model_record(self) -> Dict[str, Any]:
        return {
            "n_ctx": self.n_ctx,
            "supports_scratchpads": {
                "PASSTHROUGH": {},
            },
            "supports_tools": self.capabilities.tools,
            "supports_multimodality": self.capabilities.multimodal,
            "supports_clicks": self.capabilities.clicks,
            "supports_agent": self.capabilities.agent,
        }


class ProviderConfig(BaseModel):
    enabled: bool = True


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ProviderConfig] = Field(default_factory=dict)
    models: Dict[str, ModelConfig] = Field(default_factory=dict)


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
