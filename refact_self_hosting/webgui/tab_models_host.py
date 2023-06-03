import json
import os
import asyncio
import aiohttp

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse

from refact_self_hosting.webgui.selfhost_webutils import log
from refact_self_hosting import env

from pydantic import BaseModel, Required
from typing import Dict, Optional


__all__ = ["TabHostRouter"]


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
        fn = env.CONFIG_ENUM_GPUS

    async def _tab_host_models_assign(self, request: Request):
        fn = env.CONFIG_ENUM_GPUS
