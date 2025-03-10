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
    provider: str
    api_key: str
    enabled_models: List[str] = Field(default_factory=list)


class ThirdPartyApiConfig(BaseModel):
    providers: List[ThirdPartyProviderConfig] = Field(default_factory=list)

    @validator('providers')
    def validate_unique_providers(cls, providers):
        provider_ids = [p.provider for p in providers]
        if len(provider_ids) != len(set(provider_ids)):
            raise ValueError("Duplicate provider IDs found")
        return providers


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
        Returns a list of providers with their API keys and enabled models.
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
            # Get all providers and their models
            providers_models = litellm.models_by_provider

            # Filter models by mode = chat
            filtered_providers_models = {}
            for provider, models in providers_models.items():
                chat_models = []
                for model in models:
                    try:
                        model_info = litellm.get_model_info(model=model, custom_llm_provider=provider)
                        if model_info and model_info.get("mode") == "chat":
                            chat_models.append(model)
                    except Exception:
                        # Skip models that cause errors when getting info
                        continue

                if chat_models:
                    filtered_providers_models[provider] = chat_models

            # Get the list of all providers from litellm for display names
            all_providers = []
            for provider_id in litellm.provider_list:
                # Format provider name for display (capitalize first letter of each word)
                provider_name = provider_id.replace('_', ' ').title()
                all_providers.append({
                    "id": provider_id,
                    "name": provider_name
                })

            return JSONResponse({
                "providers": all_providers,
                "models": filtered_providers_models
            })
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
                    return ThirdPartyApiConfig.parse_obj({"providers": data})
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
            providers = []
            for provider, api_key in api_keys.items():
                providers.append({
                    "provider": provider,
                    "api_key": api_key,
                    "enabled_models": enabled_models.get(provider, [])
                })

            # Save the new configuration
            config = ThirdPartyApiConfig(providers=providers)
            self._save_config(config)

            # Rename the old configuration file to .bak
            if os.path.exists(env.CONFIG_INTEGRATIONS):
                os.rename(env.CONFIG_INTEGRATIONS, env.CONFIG_INTEGRATIONS + ".bak")
        except Exception as e:
            # If migration fails, log the error and continue
            print(f"Error migrating old configuration: {e}")