import json
import os

from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import Dict, List, Optional

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