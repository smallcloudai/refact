import json, os
import traceback

from pathlib import Path

from pydantic import BaseModel
from fastapi import APIRouter, Request
from fastapi.responses import Response

from self_hosting_machinery import env
from refact_vecdb.common.context import VDBFiles
from refact_vecdb import VDBSearchAPI
from refact_vecdb.embeds_api.embed_spads import embed_providers


__all__ = ['TabContextRouter']


class VecDBURLUpdate(BaseModel):
    url: str


class VecDBUpdateProvider(BaseModel):
    provider: str


class TabContextRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._workdir = Path(env.DIR_UNPACKED)
        self._profile_name = 'smc'

        self.add_api_route("/tab-vecdb-files-stats", self._files_stats, methods=["GET"])
        self.add_api_route("/tab-vecdb-status", self._status, methods=["GET"])
        self.add_api_route("/tab-vecdb-update-provider", self._update_provider, methods=["POST"])

    async def _update_provider(self, data: VecDBUpdateProvider, request: Request):
        with open(env.CONFIG_VECDB + ".tmp", "w") as f:
            f.write(json.dumps({"provider": data.provider}))
        VDBFiles.change_provider.touch()
        os.rename(env.CONFIG_VECDB + ".tmp", env.CONFIG_VECDB)
        gte_cfg_fn = os.path.join(env.DIR_WATCHDOG_D, "model_gte.cfg")
        if data.provider == 'gte':
            gte_cfg_template = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model_gte.cfg")))
            with open(gte_cfg_fn, "w") as f:
                j = gte_cfg_template
                del j["unfinished"]
                json.dump(j, f, indent=4)
        else:
            try:
                os.unlink(gte_cfg_fn)
            except:
                pass

    async def _status(self):
        content = {}
        try:
            vdb_search_api = VDBSearchAPI()
            search_api_status = await vdb_search_api.status(self._profile_name)
            providers = list(embed_providers.keys())

            content = {
                "status": "ok",
                "provider": search_api_status.get('provider'),
                "available_providers": providers,
                "ongoing": {},
            }
            if VDBFiles.status.exists():
                status = json.loads(VDBFiles.status.read_text())['status']
                content['status'] = status

            if VDBFiles.file_stats.exists():
                state = json.loads(VDBFiles.file_stats.read_text())
                status = ''
                if state['file_n'] != state['total']:
                    status = 'in progress'
                elif state['file_n'] == state['total']:
                    status = 'done'
                progress_text = f'{state["file_n"]}/{state["total"]}'
                progress_val = round((state['file_n'] / state['total']) * 100)
                content["ongoing"] = {'indexing': {'status': status, 'progress_text': progress_text, "progress_val": progress_val}}

            if VDBFiles.change_provider.exists():
                if content['ongoing'].get('indexing'):
                    if content['ongoing']['indexing'].get('status') != 'in progress':
                        content['ongoing']['indexing']['status'] = 'scheduled'

        except Exception as e:
            traceback.print_exc()
            content["status"] = str(e)
        print(f'status out: {content}')
        return Response(content=json.dumps(content), status_code=200)

    async def _files_stats(self):
        content = {}
        try:
            files_stats = await VDBSearchAPI().files_stats(self._profile_name)
            print(f'files_stats: {files_stats}')
            content['files_cnt'] = files_stats['files_cnt']
            content['chunks_cnt'] = files_stats['chunks_cnt']
        except Exception as e:
            traceback.format_exc()
            content["error"] = str(e)
        print(f'files_stats out: {content}')
        return Response(content=json.dumps(content))
