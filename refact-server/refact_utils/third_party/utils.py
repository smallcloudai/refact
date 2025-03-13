import json
import os
import litellm

from pydantic import BaseModel, Field
from typing import Dict, List, Any, Optional

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_webutils import log


class ThirdPartyProviderConfig(BaseModel):
    provider_name: str
    api_key: str
    enabled: bool
    enabled_models: List[str] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ThirdPartyProviderConfig] = Field(default_factory=dict)


# TODO: migration logic
def _migrate_third_party_config():
    """
    Migrate from the old configuration format to the new one.
    """
    try:
        # Load the old API keys
        api_keys = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
                api_keys = json.load(f)

        # Load the old enabled models
        enabled_models = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            with open(str(env.CONFIG_INTEGRATIONS_MODELS), "r") as f:
                enabled_models = json.load(f)

        # Create the new configuration
        providers_dict = {}
        for provider_id, api_key in api_keys.items():
            providers_dict[provider_id] = ThirdPartyProviderConfig(
                provider_name=provider_id,
                api_key=api_key,
                enabled=True,
                enabled_models=enabled_models.get(provider_id, []),
            )

        # Save the new configuration
        config = ThirdPartyApiConfig(providers=providers_dict)
        save_third_party_config(config)

        # Rename the old configuration file to .bak
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            os.rename(env.CONFIG_INTEGRATIONS, env.CONFIG_INTEGRATIONS + ".bak")
    except Exception as e:
        # If migration fails, log the error and continue
        log(f"Error migrating old configuration: {e}")


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


class ThirdPartyModel:
    PASSTHROUGH_MAX_TOKENS_LIMIT = 128_000
    COMPLETION_READY_MODELS = []

    def __init__(self, model_name: str, api_key: Optional[str] = None):
        self._model_name = model_name
        self._api_key = api_key

    @property
    def name(self) -> str:
        return self._model_name

    @property
    def api_key(self) -> str:
        return self._api_key

    @property
    def n_ctx(self) -> Optional[int]:
        if max_input_tokens := litellm.get_model_info(self._model_name).get("max_input_tokens"):
            return min(self.PASSTHROUGH_MAX_TOKENS_LIMIT, max_input_tokens)

    @property
    def supports_tools(self) -> bool:
        return litellm.supports_function_calling(self._model_name)

    @property
    def supports_multimodality(self) -> bool:
        return litellm.supports_vision(self._model_name)

    @property
    def supports_chat(self) -> bool:
        return True

    @property
    def supports_completion(self) -> bool:
        return self._model_name in self.COMPLETION_READY_MODELS

    @property
    def tokenizer_uri(self) -> str:
        # TODO: get tokenizer uri according to the provider/model
        model_path = "Xenova/gpt-4o"
        tokenizer_url = f"https://huggingface.co/{model_path}/resolve/main/tokenizer.json"
        return tokenizer_url

    # NOTE: weird function for backward compatibility
    def compose_usage_dict(self, prompt_tokens_n: int, generated_tokens_n: int) -> Dict[str, int]:
        def _pp1000t(cost_entry_name: str) -> int:
            cost = litellm.model_cost.get(self._model_name, {}).get(cost_entry_name, 0)
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
            "supports_tools": self.supports_tools,
            "supports_multimodality": self.supports_multimodality,
            # TODO: another list of supported models / setup in UI
            "supports_clicks": False,  # TODO
            "supports_agent": False,  # TODO
        }


def available_third_party_models() -> Dict[str, ThirdPartyModel]:
    config = load_third_party_config()
    models_available = {}
    for provider_id, provider_config in config.providers.items():
        if not provider_config.enabled:
            continue
        for model_name in provider_config.enabled_models:
            if model_name not in models_available:
                models_available[model_name] = ThirdPartyModel(
                    model_name,
                    provider_config.api_key,
                )
    return models_available


# TODO:
# 1. tokenizer resolve
# 2. token counting
# 3. model config

# "backend": "litellm",
# "provider": "openai",
# "tokenizer_path": "Xenova/gpt-4o",
# "resolve_as": "o1-2024-12-17",
# "T": 200_000,
# "T_out": 32_000,
# "pp1000t_prompt": 15_000,  # $15.00 / 1M tokens (2025 january)
# "pp1000t_generated": 60_000,  # $60.00 / 1M tokens (2025 january)
# "filter_caps": ["chat", "tools"],