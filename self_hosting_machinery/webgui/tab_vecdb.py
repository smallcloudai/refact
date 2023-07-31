import copy
import json
import os
import aiohttp
import time
import shutil

from pathlib import Path
from typing import Dict, Any, List

from pydantic import BaseModel, Required
from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse, StreamingResponse

from self_hosting_machinery.webgui.selfhost_webutils import log
from self_hosting_machinery import env

from refact_vecdb import VecDBAsyncAPI


__all__ = ['TabVecDBRouter']


class VecDBURLUpdate(BaseModel):
    url: str


class VecDBFindRequest(BaseModel):
    query: str
    top_k: int = 3


class TabVecDBRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._cfg_file = Path(env.CONFIG_VECDB)

        self.add_api_route("/tab-vecdb-health", self._health, methods=["GET"])
        self.add_api_route("/tab-vecdb-files-stats", self._files_stats, methods=["GET"])

        self.add_api_route("/tab-vecdb-save-url", self._save_url, methods=["POST"])
        self.add_api_route("/tab-vecdb-get-url", self._get_url, methods=["GET"])

        self.add_api_route("/tab-vecdb-enable", self._enable_vecdb, methods=["GET"])
        self.add_api_route("/tab-vecdb-disable", self._disable_vecdb, methods=["GET"])
        self.add_api_route("/tab-vecdb-is-enabled", self._is_vecdb_enabled, methods=["GET"])

        self.add_api_route("/tab-vecdb-find", self._find_req_vecdb, methods=["POST"])

    @property
    def _url(self) -> str:
        return self._get_settings_cfg().get("url", 'http://127.0.0.1:8008')

    @property
    def _vecdb_api(self) -> VecDBAsyncAPI:
        return VecDBAsyncAPI(url=self._url)

    async def _find_req_vecdb(self, post: VecDBFindRequest):
        def fmt_results(res: List[Dict[str, Any]]) -> str:
            s = ''
            for r in res:
                s += f'file_path: {r["file_path"]}\nfile_name: {r["file_name"]}\nTEXT:\n{r["text"]}\n\n'
            return s
        if not post.query:
            return Response(content=json.dumps({"text": "", "error": "ERROR: Query is required"}), status_code=500)

        try:
            res = await self._vecdb_api.find(query=post.query, top_k=post.top_k)
        except Exception as e:
            return Response(content=json.dumps({"text": "", "error": str(e)}), status_code=500)
        return Response(content=json.dumps({"text": fmt_results(res), "error": ""}), status_code=200)

    async def _enable_vecdb(self):
        self._settings_values_save({"enabled": True})
        return Response(content=json.dumps({"enabled": True}))

    async def _disable_vecdb(self):
        self._settings_values_save({"enabled": False})
        return Response(content=json.dumps({"enabled": False}))

    async def _is_vecdb_enabled(self):
        enabled = self._get_settings_cfg().get("enabled", False)
        return Response(content=json.dumps({"enabled": enabled}))

    async def _health(self):
        def content(status, display_text, error):
            return json.dumps({
                'status': status,
                'display_text': display_text,
                'error': error
            })
        try:
            await self._vecdb_api.health()
        except Exception as e:
            return Response(content=content("error", "down", str(e)), status_code=500)
        return Response(content=content("ok", "healthy ❤️", ""), status_code=200)

    async def _files_stats(self):
        try:
            files_stats = await self._vecdb_api.files_stats()
        except Exception as e:
            return Response(content=json.dumps({"error": str(e)}), status_code=500)
        return Response(content=json.dumps({
            'files_cnt': files_stats['files_cnt'],
            'chunks_cnt': files_stats['chunks_cnt']
        }))

    async def _save_url(self, post: VecDBURLUpdate):
        url = post.url
        self._settings_values_save({"url": url})
        return Response(content=json.dumps({"url": url}))

    async def _get_url(self):
        return Response(
            content=json.dumps({"url": self._url})
        )

    def _settings_values_save(self, kwargs: Dict[str, Any]) -> None:
        ex_settings = self._get_settings_cfg()
        ex_settings_copy = copy.deepcopy(ex_settings)
        for k, v in kwargs.items():
            ex_settings[k] = v

        if ex_settings == ex_settings_copy:
            return
        with self._cfg_file.open('w') as f:
            json.dump(ex_settings, f, indent=4)

    def _get_settings_cfg(self) -> Dict[str, Any]:
        if not self._cfg_file.exists():
            return {}
        with self._cfg_file.open('r') as f:
            text = f.read()
        try:
            return json.loads(text)
        except Exception:
            return {}
