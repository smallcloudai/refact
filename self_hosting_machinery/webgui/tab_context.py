import copy
import json

from pathlib import Path
from typing import Dict, Any, List, Optional

from pydantic import BaseModel
from fastapi import APIRouter, Request
from fastapi.responses import Response

from self_hosting_machinery import env

from refact_vecdb import VecDBAsyncAPI


__all__ = ['TabContextRouter']


class VecDBURLUpdate(BaseModel):
    url: str


class VecDBUpdateProvider(BaseModel):
    provider: str


class TabContextRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._cfg_file = Path(env.CONFIG_VECDB)

        self.add_api_route("/tab-vecdb-files-stats", self._files_stats, methods=["GET"])
        self.add_api_route("/tab-vecdb-status", self._status, methods=["GET"])
        self.add_api_route('/tab-vecdb-update-provider', self._update_provider, methods=["POST"])

    async def _update_provider(self, data: VecDBUpdateProvider, request: Request):
        provider = data.provider
        with Path(env.DIR_UNPACKED).joinpath('vecdb_update_provider.json').open('w') as f:
            f.write(json.dumps({'provider': provider}))

    @property
    def _url(self) -> str:
        return 'http://0.0.0.0:8009'

    @property
    def _vecdb_api(self) -> VecDBAsyncAPI:
        return VecDBAsyncAPI(url=self._url)

    async def _status(self):
        try:
            status_resp = await self._vecdb_api.status()
            if Path(env.FLAG_VECDB_CHANGE_MODEL).exists():
                status_resp['ongoing'] = {'indexing': {'status': 'scheduled'}}
        except Exception as e:
            content = json.dumps({
                'status': str(e),
            })
        else:
            print(f'status: {status_resp}')
            content = json.dumps({
                "status": status_resp.get('status'),
                "embed_model": status_resp.get('embed_model'),
                "provider": status_resp.get('provider'),
                "available_providers": status_resp.get('available_providers'),
                "ongoing": status_resp.get('ongoing', {}),
            })

        return Response(content=content, status_code=200)

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
