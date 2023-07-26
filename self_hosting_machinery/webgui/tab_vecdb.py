import json
import os
import aiohttp
import time
import shutil

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse, StreamingResponse

from self_hosting_machinery.webgui.selfhost_webutils import log
from self_hosting_machinery.webgui.tab_finetune import get_finetune_runs
from self_hosting_machinery import env

from pydantic import BaseModel, Required
from typing import Dict, Optional


class TabVecDBRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/health", self._health, methods=["GET"])

    async def _health(self, request: Request):
        pass

    async def _files_count(self, request: Request):
        pass
