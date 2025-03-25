from fastapi import APIRouter, UploadFile
from fastapi.responses import JSONResponse

from refact_utils.third_party.utils import ThirdPartyApiConfig
from refact_utils.third_party.utils import load_third_party_config
from refact_utils.third_party.utils import save_third_party_config
from refact_utils.third_party.utils import get_provider_models
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

        # Tokenizer routes
        self.add_api_route("/tab-third-party-apis-get-tokenizers", self._tab_third_party_apis_get_tokenizers, methods=["GET"])
        self.add_api_route("/tab-third-party-apis-upload-tokenizer", self._tab_third_party_apis_upload_tokenizer, methods=["POST"])
        self.add_api_route("/tab-third-party-apis-delete-tokenizer", self._tab_third_party_apis_delete_tokenizer, methods=["POST"])

    async def _tab_third_party_apis_get(self):
        config = load_third_party_config()
        return JSONResponse(config.dict())

    async def _tab_third_party_apis_save(self, config: ThirdPartyApiConfig):
        try:
            save_third_party_config(config)
            self._models_assigner.models_to_watchdog_configs()
            return JSONResponse({"status": "OK"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=400)

    async def _tab_third_party_apis_get_providers(self):
        return JSONResponse(get_provider_models())

    async def _tab_third_party_apis_get_tokenizers(self):
        # TODO: implement
        return JSONResponse([])

    async def _tab_third_party_apis_upload_tokenizer(self, file: UploadFile):
        # TODO: implement, see _upload_lora, _upload_lora_url in tab_loras.py
        return JSONResponse("OK", status_code=200)

    async def _tab_third_party_apis_delete_tokenizer(self, tokenizer_id: str):
        return JSONResponse("OK", status_code=200)
