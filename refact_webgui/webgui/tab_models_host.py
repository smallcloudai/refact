import json

from fastapi import APIRouter, Query, HTTPException
from fastapi.responses import Response, JSONResponse

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_active_loras
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner

from pydantic import BaseModel, validator
from typing import Dict


__all__ = ["TabHostRouter"]


class ModifyLorasPost(BaseModel):
    model: str
    mode: str
    run_id: str
    checkpoint: str

    @validator('mode')
    def validate_mode(cls, v: str):
        if v not in ['add', 'remove']:
            raise HTTPException(status_code=400, detail="mode must be 'add' or 'remove'")
        return v


class TabHostModelRec(BaseModel):
    gpus_shard: int = Query(default=1, ge=1, le=4)
    share_gpu: bool = False


class TabHostModelsAssign(BaseModel):
    model_assign: Dict[str, TabHostModelRec] = {}
    completion: str

    # integrations
    openai_api_enable: bool = False
    anthropic_api_enable: bool = False


class TabHostRouter(APIRouter):
    def __init__(self, model_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._model_assigner = model_assigner
        self.add_api_route("/tab-host-modify-loras", self._modify_loras, methods=["POST"])
        self.add_api_route("/tab-host-have-gpus", self._tab_host_have_gpus, methods=["GET"])
        self.add_api_route("/tab-host-models-get", self._tab_host_models_get, methods=["GET"])
        self.add_api_route("/tab-host-models-assign", self._tab_host_models_assign, methods=["POST"])

    async def _modify_loras(self, post: ModifyLorasPost):
        active_loras = get_active_loras(self._model_assigner.models_db)

        lora_model_cfg = active_loras.get(post.model, {})
        lora_model_cfg.setdefault('loras', [])

        if post.mode == "remove":
            lora_model_cfg['loras'] = [l for l in lora_model_cfg['loras'] if l['run_id'] != post.run_id and l['checkpoint'] != post.checkpoint]
        if post.mode == "add":
            if (post.run_id, post.checkpoint) not in [(l['run_id'], l['checkpoint']) for l in lora_model_cfg['loras']]:
                lora_model_cfg['loras'].append({
                    'run_id': post.run_id,
                    'checkpoint': post.checkpoint,
                })
            else:
                raise HTTPException(status_code=400, detail=f"lora {post.run_id} {post.checkpoint} already exists")

        active_loras[post.model] = lora_model_cfg

        with open(env.CONFIG_ACTIVE_LORA, "w") as f:
            json.dump(active_loras, f, indent=4)

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
        return JSONResponse("OK")
