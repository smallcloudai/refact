from fastapi import APIRouter, UploadFile, Form, File
from fastapi.responses import JSONResponse

from pydantic import BaseModel

from refact_utils.third_party.utils.configs import ThirdPartyApiConfig
from refact_utils.third_party.utils.models import load_third_party_config
from refact_utils.third_party.utils.models import save_third_party_config
from refact_utils.third_party.utils.models import get_provider_models
from refact_utils.third_party.utils.tokenizers import get_default_tokenizers
from refact_utils.third_party.utils.tokenizers import get_tokenizers
from refact_utils.third_party.utils.tokenizers import upload_tokenizer
from refact_utils.third_party.utils.tokenizers import delete_tokenizer


__all__ = ["TabThirdPartyApisRouter"]


class DeleteTokenizer(BaseModel):
    tokenizer_id: str


class TabThirdPartyApisRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        # Models config routes
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
            return JSONResponse({"status": "OK"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=400)

    async def _tab_third_party_apis_get_providers(self):
        return JSONResponse(get_provider_models())

    async def _tab_third_party_apis_get_tokenizers(self):
        try:
            return JSONResponse({
                "defaults": get_default_tokenizers(),
                "uploaded": get_tokenizers(),
            })
        except Exception as e:
            return JSONResponse({"detail": f"Error get tokenizers: {e}"}, status_code=400)

    async def _tab_third_party_apis_upload_tokenizer(
            self,
            tokenizer_id: str = Form(...),
            file: UploadFile = File(...)
    ):
        try:
            await upload_tokenizer(tokenizer_id, file)
            return JSONResponse("OK", status_code=200)
        except Exception as e:
            return JSONResponse({"detail": f"Error uploading tokenizer: {e}"}, status_code=400)

    async def _tab_third_party_apis_delete_tokenizer(self, post: DeleteTokenizer):
        try:
            delete_tokenizer(post.tokenizer_id)
            return JSONResponse("OK", status_code=200)
        except Exception as e:
            return JSONResponse({"detail": f"Error deleting tokenizer: {e}"}, status_code=400)
