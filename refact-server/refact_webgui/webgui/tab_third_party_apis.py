from fastapi import APIRouter, UploadFile
from fastapi.responses import JSONResponse

from pydantic import BaseModel
from pathlib import Path

from refact_utils.scripts import env
from refact_utils.third_party.utils import ThirdPartyApiConfig
from refact_utils.third_party.utils import load_third_party_config
from refact_utils.third_party.utils import save_third_party_config
from refact_utils.third_party.utils import get_provider_models

from refact_webgui.webgui.tab_loras import write_to_file
from refact_webgui.webgui.tab_loras import rm


__all__ = ["TabThirdPartyApisRouter"]


class UploadViaURL(BaseModel):
    url: str


class TabThirdPartyApisRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

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
            # self._models_assigner.models_to_watchdog_configs()
            return JSONResponse({"status": "OK"})
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=400)

    async def _tab_third_party_apis_get_providers(self):
        return JSONResponse(get_provider_models())

    @property
    def _tokenizers_dir(self) -> Path:
        return Path(env.DIR_TOKENIZERS)

    async def _tab_third_party_apis_get_tokenizers(self):
        return JSONResponse([
            filename.name
            for filename in self._tokenizers_dir.iterdir()
            if filename.name.endswith(".json")
        ])

    async def _tab_third_party_apis_upload_tokenizer(self, file: UploadFile):
        filename = self._tokenizers_dir / file.filename
        try:
            if not self._tokenizers_dir.exists():
                raise RuntimeError(f"no tokenizers dir `{self._tokenizers_dir}`")

            if not file.filename.endswith(".json"):
                return JSONResponse(
                    status_code=400,
                    content={"detail": "Tokenizer file should have .json extension"},
                )

            if (resp := await write_to_file(str(self._tokenizers_dir), file)).status_code != 200:
                return resp

            return JSONResponse("OK", status_code=200)
        except Exception as e:
            filename.unlink(missing_ok=True)
            return JSONResponse({"detail": f"Error uploading tokenizer: {e}"}, status_code=400)

    async def _tab_third_party_apis_delete_tokenizer(self, tokenizer_id: str):
        try:
            tokenizer_path = self._tokenizers_dir / f"{tokenizer_id}.json"
            if tokenizer_path.exists():
                tokenizer_path.unlink()
            return JSONResponse("OK", status_code=200)
        except Exception as e:
            return JSONResponse({"detail": f"Error deleting tokenizer: {e}"}, status_code=400)
