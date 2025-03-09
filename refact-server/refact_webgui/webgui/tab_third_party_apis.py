import json
import os
import litellm

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import Dict, List, Optional, Any

class AddModelRequest(BaseModel):
    providerId: str
    modelId: str

class AddProviderRequest(BaseModel):
    providerId: str
    providerName: str
    apiKey: Optional[str] = None

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner


__all__ = ["TabThirdPartyApisRouter"]


class TabThirdPartyApisRouter(APIRouter):
    class ApiKeys(BaseModel):
        openai_api_key: Optional[str] = None
        anthropic_api_key: Optional[str] = None
        groq_api_key: Optional[str] = None
        cerebras_api_key: Optional[str] = None
        gemini_api_key: Optional[str] = None
        xai_api_key: Optional[str] = None
        deepseek_api_key: Optional[str] = None

    # class EnabledModels(BaseModel):
    #     __root__: Dict[str, List[str]]

    def __init__(self, models_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._models_assigner = models_assigner
        self.add_api_route("/tab-third-party-apis-get", self._tab_third_party_apis_get, methods=["GET"])
        self.add_api_route("/tab-third-party-apis-save-keys", self._tab_third_party_apis_save_keys, methods=["POST"])
        self.add_api_route("/tab-third-party-apis-save-models", self._tab_third_party_apis_save_models, methods=["POST"])
        self.add_api_route("/tab-third-party-apis-get-providers", self._tab_third_party_apis_get_providers, methods=["GET"])
        self.add_api_route("/tab-third-party-apis-get-all-providers", self._tab_third_party_apis_get_all_providers, methods=["GET"])
        self.add_api_route("/tab-third-party-apis-add-provider", self._tab_third_party_apis_add_provider, methods=["POST"])
        self.add_api_route("/tab-third-party-apis-add-model", self._tab_third_party_apis_add_model, methods=["POST"])
        # self.add_api_route("/tab-third-party-apis-get-model-info", self._tab_third_party_apis_get_model_info, methods=["GET"])

    async def _tab_third_party_apis_get(self):
        # Get API keys
        api_keys = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
                api_keys = json.load(f)

        # Get enabled models
        enabled_models = {}
        if os.path.exists(env.CONFIG_ENABLED_MODELS):
            with open(str(env.CONFIG_ENABLED_MODELS), "r") as f:
                enabled_models = json.load(f)

        return JSONResponse({
            "apiKeys": api_keys,
            "enabledModels": enabled_models
        })

    async def _tab_third_party_apis_save_keys(self, data: Dict[str, str]):
        # Save API keys
        with open(env.CONFIG_INTEGRATIONS + ".tmp", "w") as f:
            json.dump(data, f, indent=4)
        os.rename(env.CONFIG_INTEGRATIONS + ".tmp", env.CONFIG_INTEGRATIONS)

        # Update model assigner
        self._models_assigner.models_to_watchdog_configs()

        return JSONResponse({"status": "OK"})

    async def _tab_third_party_apis_save_models(self, data: Dict[str, List[str]]):
        # Save enabled models
        with open(env.CONFIG_ENABLED_MODELS + ".tmp", "w") as f:
            json.dump(data, f, indent=4)
        os.rename(env.CONFIG_ENABLED_MODELS + ".tmp", env.CONFIG_ENABLED_MODELS)

        # Update model assigner
        self._models_assigner.models_to_watchdog_configs()

        return JSONResponse({"status": "OK"})

    async def _tab_third_party_apis_get_providers(self):
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

            return JSONResponse(filtered_providers_models)
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)

    async def _tab_third_party_apis_get_all_providers(self):
        try:
            # Get all available providers from litellm
            all_providers = []

            # Get the list of all providers from litellm
            for provider_id in litellm.provider_list:
                # Format provider name for display (capitalize first letter of each word)
                provider_name = provider_id.replace('_', ' ').title()
                all_providers.append({
                    "id": provider_id,
                    "name": provider_name
                })

            return JSONResponse(all_providers)
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)

    async def _tab_third_party_apis_add_provider(self, request: AddProviderRequest):
        try:
            provider_id = request.providerId
            provider_name = request.providerName
            api_key = request.apiKey

            # Save the API key if provided
            if api_key:
                # Get existing API keys
                api_keys = {}
                if os.path.exists(env.CONFIG_INTEGRATIONS):
                    with open(str(env.CONFIG_INTEGRATIONS), "r") as f:
                        api_keys = json.load(f)

                # Add the new API key
                api_keys[provider_id] = api_key

                # Save the updated API keys
                with open(env.CONFIG_INTEGRATIONS + ".tmp", "w") as f:
                    json.dump(api_keys, f, indent=4)
                os.rename(env.CONFIG_INTEGRATIONS + ".tmp", env.CONFIG_INTEGRATIONS)

                # Update model assigner
                self._models_assigner.models_to_watchdog_configs()

            # Return success
            return JSONResponse({"status": "OK", "message": "Provider added successfully"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)

    async def _tab_third_party_apis_add_model(self, request: AddModelRequest):
        try:
            provider_id = request.providerId
            model_id = request.modelId

            # Get the current provider models
            providers_models = {}
            try:
                import litellm
                providers_models = litellm.models_by_provider
            except (ImportError, Exception):
                # If litellm is not available, use an empty dict
                pass

            # Add the model to the provider's models if it doesn't exist
            if provider_id not in providers_models:
                providers_models[provider_id] = []

            if model_id not in providers_models[provider_id]:
                providers_models[provider_id].append(model_id)

            # Return success
            return JSONResponse({"status": "OK", "message": "Model added successfully"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)

    # async def _tab_third_party_apis_get_model_info(self, model_name: str, provider_name: str):
    #     try:
    #         model_info = litellm.get_model_info(model_name=model_name, provider_name=provider_name)
    #         return JSONResponse(model_info)
    #     except Exception as e:
    #         return JSONResponse({"error": str(e)}, status_code=500)
