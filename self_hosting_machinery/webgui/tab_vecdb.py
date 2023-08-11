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

        self.add_api_route("/tab-vecdb-upload-files", self._tab_vecdb_upload_files, methods=["GET"])

    async def _tab_vecdb_upload_files(self):
        try:
            unpacked_dir = Path(env.DIR_UNPACKED)

            train_set_filtered = unpacked_dir / 'train_set_filtered.jsonl'
            test_set_filtered = unpacked_dir / 'test_set_filtered.jsonl'

            if not train_set_filtered.is_file() or not test_set_filtered.is_file():
                return Response(content=json.dumps({"status": "train set or test set not found"}), status_code=200)

            train_set = [json.loads(line) for line in train_set_filtered.open('r')]
            test_set = [json.loads(line) for line in test_set_filtered.open('r')]

            file_paths: List[Path] = [unpacked_dir.joinpath(d['path']) for d in [*train_set, *test_set] if d['path']]
            file_paths: List[str] = [str(p) for p in file_paths if p.is_file()]
            if not file_paths:
                return Response(content=json.dumps({"status": "no files to upload"}), status_code=200)

            with unpacked_dir.joinpath('vecdb_paths_upload.json').open('w') as f:
                f.write(json.dumps(file_paths))

            with Path(env.FLAG_VECDB_FILES_UPLOAD).open('w') as f:
                f.write('')

            return Response(content=json.dumps({"status": "scheduled"}), status_code=200)
        except Exception as e:
            return Response(content=json.dumps({"status": str(e)}), status_code=200)


    @property
    def _url(self) -> str:
        return 'http://0.0.0.0:8009'

    @property
    def _vecdb_api(self) -> VecDBAsyncAPI:
        return VecDBAsyncAPI(url=self._url)

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
            return Response(content=content("error", str(e), str(e)), status_code=200)
        return Response(content=content("ok", "healthy ❤️", ""), status_code=200)

    async def _files_stats(self):
        try:
            files_stats = await self._vecdb_api.files_stats()
        except Exception as e:
            return Response(content=json.dumps({"error": str(e)}), status_code=200)
        return Response(content=json.dumps({
            'files_cnt': files_stats['files_cnt'],
            'chunks_cnt': files_stats['chunks_cnt']
        }))

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
