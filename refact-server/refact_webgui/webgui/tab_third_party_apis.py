import json
import os
import litellm

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field, validator
from typing import Dict, List, Optional, Any

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner


__all__ = ["TabThirdPartyApisRouter"]


class ThirdPartyProviderConfig(BaseModel):
    provider_name: str
    api_key: str
    enabled_models: List[str] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ThirdPartyProviderConfig] = Field(default_factory=dict)


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
        config = self._load_config()
        return JSONResponse(config.dict())

    async def _tab_third_party_apis_save(self, data: Dict[str, Any]):
        """
        Save the third-party API configuration.
        Expects a dictionary that can be parsed into a ThirdPartyApiConfig.
        """
        try:
            # Validate the data
            config = ThirdPartyApiConfig.parse_obj(data)

            # Save the configuration
            self._save_config(config)
            
            # Update model assigner
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

    def _load_config(self) -> ThirdPartyApiConfig:
        """
        Load the third-party API configuration from the file.
        If the file doesn't exist or is invalid, return an empty configuration.
        """
        # Check if the old config exists and migrate it
        if os.path.exists(env.CONFIG_INTEGRATIONS) and not os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            self._migrate_old_config()

        # Load the configuration
        if os.path.exists(env.CONFIG_INTEGRATIONS_MODELS):
            try:
                with open(str(env.CONFIG_INTEGRATIONS_MODELS), "r") as f:
                    data = json.load(f)
                    # Convert from list to dict if needed (for backward compatibility)
                    if isinstance(data, list):
                        providers_dict = {}
                        for provider_config in data:
                            provider_id = provider_config.pop("provider", None)
                            if provider_id:
                                # Rename provider to provider_name if it exists
                                provider_config["provider_name"] = provider_config.get("provider_name", provider_id)
                                providers_dict[provider_id] = provider_config
                        return ThirdPartyApiConfig(providers=providers_dict)
                    else:
                        # If already a dict, ensure provider_name is set
                        for provider_id, config in data.items():
                            if "provider_name" not in config:
                                config["provider_name"] = provider_id
                        return ThirdPartyApiConfig(providers=data)
            except (json.JSONDecodeError, ValueError):
                # If the file is invalid, return an empty configuration
                return ThirdPartyApiConfig()

        return ThirdPartyApiConfig()

    def _save_config(self, config: ThirdPartyApiConfig):
        """
        Save the third-party API configuration to the file.
        """
        # Create the directory if it doesn't exist
        os.makedirs(os.path.dirname(env.CONFIG_INTEGRATIONS_MODELS), exist_ok=True)

        # Save the configuration
        with open(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", "w") as f:
            json.dump(config.dict()["providers"], f, indent=4)
        os.rename(env.CONFIG_INTEGRATIONS_MODELS + ".tmp", env.CONFIG_INTEGRATIONS_MODELS)

    def _migrate_old_config(self):
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
                providers_dict[provider_id] = {
                    "provider_name": provider_id,
                    "api_key": api_key,
                    "enabled_models": enabled_models.get(provider_id, [])
                }

            # Save the new configuration
            config = ThirdPartyApiConfig(providers=providers_dict)
            self._save_config(config)

            # Rename the old configuration file to .bak
            if os.path.exists(env.CONFIG_INTEGRATIONS):
                os.rename(env.CONFIG_INTEGRATIONS, env.CONFIG_INTEGRATIONS + ".bak")
        except Exception as e:
            # If migration fails, log the error and continue
            print(f"Error migrating old configuration: {e}")