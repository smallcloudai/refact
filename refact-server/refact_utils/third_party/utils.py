import json
import os
import litellm

from pydantic import BaseModel, Field
from typing import Dict, List, Any, Optional

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_webutils import log


class CustomModelConfig(BaseModel):
    api_key: str
    n_ctx: int
    supports_tools: bool
    supports_multimodality: bool
    tokenizer_uri: Optional[str]


class ModelConfig(BaseModel):
    model_name: str
    supports_agentic: bool = False
    supports_clicks: bool = False
    custom_model_config: Optional[CustomModelConfig] = None


class ThirdPartyProviderConfig(BaseModel):
    provider_name: str
    enabled: bool
    api_key: Optional[str] = None
    enabled_models: List[ModelConfig] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ThirdPartyProviderConfig] = Field(default_factory=dict)


# TODO: migration logic
# def _migrate_third_party_config():
#     """
#     Migrate from the old configuration format to the new one.
#     """
#     try:
#         # Load the old API keys
#         api_keys = {}
#         if os.path.exists(env.CONFIG_INTEGRATIONS):
#             with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
#                 api_keys = json.load(f)
#
#         # Load the old enabled models
#         enabled_models = {}
#         if os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
#             with open(str(env.CONFIG_INTEGRATIONS_MODELS), "r") as f:
#                 enabled_models = json.load(f)
#
#         # Create the new configuration
#         providers_dict = {}
#         for provider_id, api_key in api_keys.items():
#             providers_dict[provider_id] = ThirdPartyProviderConfig(
#                 provider_name=provider_id,
#                 api_key=api_key,
#                 enabled=True,
#                 enabled_models=enabled_models.get(provider_id, []),
#             )
#
#         # Save the new configuration
#         config = ThirdPartyApiConfig(providers=providers_dict)
#         save_third_party_config(config)
#
#         # Rename the old configuration file to .bak
#         if os.path.exists(env.CONFIG_INTEGRATIONS):
#             os.rename(env.CONFIG_INTEGRATIONS, env.CONFIG_INTEGRATIONS + ".bak")
#     except Exception as e:
#         # If migration fails, log the error and continue
#         log(f"Error migrating old configuration: {e}")


def load_third_party_config() -> ThirdPartyApiConfig:
    """
    Load the third-party API configuration from the file.
    If the file doesn't exist or is invalid, return an empty configuration.
    """
    # Check if the old config exists and migrate it
    # if os.path.exists(env.CONFIG_INTEGRATIONS) and not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
    #     _migrate_third_party_config()

    try:
        if not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            raise FileNotFoundError(f"No third party config found")
        with open(env.CONFIG_INTEGRATIONS_MODELS, "r") as f:
            data = json.load(f)
        return ThirdPartyApiConfig.model_validate(data)
    except Exception as e:
        log(f"Can't read third-party providers config, fallback to empty: {e}")
        return ThirdPartyApiConfig()


def save_third_party_config(config: ThirdPartyApiConfig):
    """
    Save the third-party API configuration to the file.
    """
    # Create the directory if it doesn't exist
    os.makedirs(os.path.dirname(env.CONFIG_INTEGRATIONS_MODELS), exist_ok=True)

    # Save the configuration
    with open(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", "w") as f:
        json.dump(config.model_dump(), f, indent=4)
    os.rename(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", env.CONFIG_INTEGRATIONS_MODELS)


def _supports_chat(model_name: str) -> bool:
    model_info = litellm.get_model_info(model=model_name)
    return model_info and model_info.get("mode") == "chat"


def _get_context_size(model_name: str) -> Optional[int]:
    model_info = litellm.get_model_info(model_name)
    return model_info.get("max_input_tokens", 8192)


class ThirdPartyModel:
    PASSTHROUGH_MAX_TOKENS_LIMIT = 128_000
    COMPLETION_READY_MODELS = []

    def __init__(
            self,
            model_config: ModelConfig,
            api_key: Optional[str] = None,
    ):
        self._model_config = model_config
        if model_config.custom_model_config is None:
            self._api_key = api_key
            self._n_ctx = min(_get_context_size(self.name), self.PASSTHROUGH_MAX_TOKENS_LIMIT)
            self._supports_chat = _supports_chat(self.name)
            self._supports_tools = bool(litellm.supports_function_calling(self.name))
            self._supports_multimodality = bool(litellm.supports_vision(self.name))
        else:
            self._api_key = model_config.custom_model_config.api_key
            self._n_ctx = model_config.custom_model_config.n_ctx
            self._supports_chat = True  # custom models are only for chat
            self._supports_tools = bool(model_config.custom_model_config.supports_tools)
            self._supports_multimodality = bool(model_config.custom_model_config.supports_multimodality)
        assert self._n_ctx is not None, "no context size"

    @property
    def name(self) -> str:
        return self._model_config.model_name

    @property
    def api_key(self) -> str:
        return self._api_key

    @property
    def n_ctx(self) -> int:
        return self._n_ctx

    @property
    def supports_chat(self) -> bool:
        return self._supports_chat

    @property
    def supports_completion(self) -> bool:
        return self.name in self.COMPLETION_READY_MODELS

    @property
    def tokenizer_uri(self) -> str:
        # TODO: get tokenizer uri according to the provider/model
        model_path = "Xenova/gpt-4o"
        tokenizer_url = f"https://huggingface.co/{model_path}/resolve/main/tokenizer.json"
        return tokenizer_url

    # NOTE: weird function for backward compatibility
    def compose_usage_dict(self, prompt_tokens_n: int, generated_tokens_n: int) -> Dict[str, int]:
        def _pp1000t(cost_entry_name: str) -> int:
            cost = litellm.model_cost.get(self.name, {}).get(cost_entry_name, 0)
            return int(cost * 1_000_000 * 1_000)
        return {
            "pp1000t_prompt": _pp1000t("input_cost_per_token"),
            "pp1000t_generated": _pp1000t("output_cost_per_token"),
            "metering_prompt_tokens_n": prompt_tokens_n,
            "metering_generated_tokens_n": generated_tokens_n,
        }

    def to_completion_model_record(self) -> Dict[str, Any]:
        assert self.supports_completion
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
            "supports_tools": self._supports_tools,
            "supports_multimodality": self._supports_multimodality,
            "supports_clicks": self._model_config.supports_clicks,
            "supports_agent": self._model_config.supports_agentic,
        }


def available_third_party_models() -> Dict[str, ThirdPartyModel]:
    config = load_third_party_config()
    models_available = {}
    for provider_id, provider_config in config.providers.items():
        if not provider_config.enabled:
            continue
        for model_config in provider_config.enabled_models:
            if model_config.model_name not in models_available:
                try:
                    models_available[model_config.model_name] = ThirdPartyModel(
                        model_config,
                        provider_config.api_key,
                    )
                except Exception as e:
                    log(f"model listed as available but it's not supported: {e}")
    return models_available


def get_provider_models() -> Dict[str, List[str]]:
    filtered_providers_models = {}
    for provider in litellm.provider_list:
        provider_chat_models = []
        provider_name = str(provider.value)
        provider_models = litellm.models_by_provider.get(provider_name, [])
        for model_name in provider_models:
            try:
                model = ThirdPartyModel(ModelConfig(model_name=model_name))
                if model.supports_chat:
                    provider_chat_models.append(model_name)
            except Exception:
                continue
        if not provider_models or provider_chat_models:
            filtered_providers_models[provider_name] = provider_chat_models
    return filtered_providers_models
