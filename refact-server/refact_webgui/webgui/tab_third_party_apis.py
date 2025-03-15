import litellm

from fastapi import APIRouter
from fastapi.responses import JSONResponse

from refact_utils.third_party.utils import ThirdPartyApiConfig
from refact_utils.third_party.utils import load_third_party_config
from refact_utils.third_party.utils import save_third_party_config
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner


__all__ = ["TabThirdPartyApisRouter"]


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
