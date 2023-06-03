import json
import os
import asyncio
import aiohttp

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse

from refact_self_hosting.webgui.selfhost_webutils import log
from refact_self_hosting import env
from refact_self_hosting import known_models
from code_contrast.model_caps import modelcap_records

from pydantic import BaseModel, Required
from typing import Dict, Optional


__all__ = ["TabHostRouter"]


class TabHostModelRec(BaseModel):
    gpus_min: int = Query(default=0, ge=0, le=8)
    gpus_max: int = Query(default=8, ge=0, le=8)


class TabHostModelsAssign(BaseModel):
    model_assign: Dict[str, TabHostModelRec] = {}


class TabHostRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-host-have-gpus", self._tab_host_have_gpus, methods=["GET"])
        self.add_api_route("/tab-host-models-get", self._tab_host_models_get, methods=["GET"])
        self.add_api_route("/tab-host-models-assign", self._tab_host_models_assign, methods=["POST"])

    async def _tab_host_have_gpus(self, request: Request):
        fn = env.CONFIG_ENUM_GPUS
        if os.path.exists(fn):
            j = json.load(open(fn, "r"))
        else:
            j = {"gpus": []}
        return Response(json.dumps(j, indent=4) + "\n")

    async def _tab_host_models_get(self, request: Request):
        j = {"models": []}
        for k, rec in known_models.models_mini_db.items():
            if rec.get("hidden", False):
                continue
            j["models"].append({
                "name": k,
                "has_chat": not not rec["chat_scratchpad_class"],
                "has_toolbox": False,
            })
            k_filter_caps = rec["filter_caps"]
            for rec in modelcap_records.db:
                rec_models = rec.model
                if not isinstance(rec_models, list):
                    rec_models = [rec_models]
                for test in rec_models:
                    if test in k_filter_caps:
                        # print("model %s has toolbox because %s" % (k, rec.function_name))
                        j["models"][-1]["has_toolbox"] = True
                        break
        fn = env.CONFIG_INFERENCE
        if os.path.exists(fn):
            j2 = json.load(open(fn, "r"))
        else:
            j2 = {"model_assign": {}}
        j.update(j2)
        return Response(json.dumps(j, indent=4) + "\n")

    async def _tab_host_models_assign(self, post: TabHostModelsAssign, request: Request):
        fn = env.CONFIG_INFERENCE
        with open(fn, "w") as f:
            json.dump(post.dict(), f, indent=4)
        return JSONResponse("OK")
