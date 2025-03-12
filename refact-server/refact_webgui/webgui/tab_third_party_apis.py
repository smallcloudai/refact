import json
import os
import litellm

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field
from typing import Dict, List

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_webutils import log
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner


__all__ = ["TabThirdPartyApisRouter"]


class ThirdPartyProviderConfig(BaseModel):
    provider_name: str
    api_key: str
    enabled: bool
    enabled_models: List[str] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ThirdPartyProviderConfig] = Field(default_factory=dict)


# TODO: migration logic
def migrate_third_party_config():
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
    #     migrate_third_party_config()

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


class TabThirdPartyApisRouter(APIRouter):
    def __init__(self, models_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._models_assigner = models_assigner

        # Add API routes
        self.add_api_route("/tab-third-party-apis-get", self._tab_third_party_apis_get, methods=["GET"])
        self.add_api_route("/tab-third-party-apis-save", self._tab_third_party_apis_save, methods=["POST"])
        self.add_api_route("/tab-third-party-apis-get-providers", self._tab_third_party_apis_get_providers, methods=["GET"])

    async def _tab_third_party_apis_get(self):
        """
        Get the current third-party API configuration.
        Returns a dictionary of providers with their API keys and enabled models.
        """
        config = load_third_party_config()
        return JSONResponse(config.dict())

    async def _tab_third_party_apis_save(self, config: ThirdPartyApiConfig):
        """
        Save the third-party API configuration.
        Expects a dictionary that can be parsed into a ThirdPartyApiConfig.
        """
        try:
            save_third_party_config(config)
            self._models_assigner.models_to_watchdog_configs()
            return JSONResponse({"status": "OK"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=400)

    async def _tab_third_party_apis_get_providers(self):
        """
        Get all available providers and their models from litellm.
        Filters models to only include chat models.
        """
        try:
            providers_models = litellm.models_by_provider

            filtered_providers_models = {}
            for provider, models in providers_models.items():
                chat_models = []
                for model in models:
                    try:
                        model_info = litellm.get_model_info(model=model, custom_llm_provider=provider)
                        if model_info and model_info.get("mode") == "chat":
                            chat_models.append(model)
                    except Exception:
                        continue

                if chat_models:
                    filtered_providers_models[provider] = chat_models

            return JSONResponse(filtered_providers_models)
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)
