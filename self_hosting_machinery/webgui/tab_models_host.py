import json

from fastapi import APIRouter, Query
from fastapi.responses import Response, JSONResponse

from self_hosting_machinery.webgui.selfhost_model_assigner import ModelAssigner

from pydantic import BaseModel
from typing import Dict


__all__ = ["TabHostRouter"]


class TabHostModelRec(BaseModel):
    gpus_shard: int = Query(default=1, ge=1, le=4)
    share_gpu: bool = False


class TabHostModelsAssign(BaseModel):
    model_assign: Dict[str, TabHostModelRec] = {}
    completion: str
    openai_api_enable: bool = False


class TabHostRouter(APIRouter):
    def __init__(self, model_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._model_assigner = model_assigner
        self.add_api_route("/tab-host-have-gpus", self._tab_host_have_gpus, methods=["GET"])
        self.add_api_route("/tab-host-models-get", self._tab_host_models_get, methods=["GET"])
        self.add_api_route("/tab-host-models-assign", self._tab_host_models_assign, methods=["POST"])

    async def _tab_host_have_gpus(self):
        return Response(json.dumps(self._model_assigner.gpus, indent=4) + "\n")

    async def _tab_host_models_get(self):
        return Response(json.dumps({
            **self._model_assigner.models_info,
            **self._model_assigner.model_assignment,
        }, indent=4) + "\n")

    async def _tab_host_models_assign(self, post: TabHostModelsAssign):
        validated = post.dict()
        current_completion_model = validated.get("completion", "")
        if not current_completion_model or current_completion_model not in post.model_assign:
            for info in self._model_assigner.models_info["models"]:
                if info["has_completion"] and info["name"] in post.model_assign:
                    validated["completion"] = info["name"]
                    break
            else:
                validated["completion"] = ""

        self._model_assigner.models_to_watchdog_configs(validated)
        self._model_assigner.restart_lsp()
        return JSONResponse("OK")
