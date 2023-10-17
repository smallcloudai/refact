import json, os
import traceback

from pathlib import Path

from pydantic import BaseModel
from fastapi import APIRouter, Request

from self_hosting_machinery import env
from refact_vecdb.common.context import VDBFiles
from refact_vecdb import VDBSearchAPI
from refact_vecdb.embeds_api.embed_spads import models as embed_providers


__all__ = ['TabContextRouter']


class VecDBURLUpdate(BaseModel):
    url: str


class VecDBUpdateProvider(BaseModel):
    provider: str


class TabContextRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._workdir = Path(env.DIR_UNPACKED)

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

    async def _status(self, account: str = 'XXX'):
        content = {}
        try:
            vdb_search_api = VDBSearchAPI()
            search_api_status = await vdb_search_api.status(account)
            content.update({
                "status": "ok",
                "provider": search_api_status.get('provider'),
                "change_provider_flag": os.path.exists(env.FLAG_VECDB_CHANGE_PROVIDER),
                "available_providers": embed_providers,
                "ongoing": {},
            })

            if os.path.exists(env.CONFIG_VECDB_STATUS):
                status = json.loads(open(env.CONFIG_VECDB_STATUS).read())
                content['status'] = status['status']

            if os.path.exists(env.CONFIG_VECDB_FILE_STATS):
                state = json.loads(open(env.CONFIG_VECDB_FILE_STATS).read())  # "file_n", "total"
                content["ongoing"] = {'indexing': state}

        except Exception as e:
            traceback.print_exc()
            content["status"] = str(e)

        return content

    async def _files_stats(self, account: str = 'XXX'):
        try:
            files_stats = await VDBSearchAPI().files_stats(account)
            return {
                "files_cnt": files_stats['files_cnt'],
                "chunks_cnt": files_stats['chunks_cnt']
            }
        except Exception as e:
            traceback.format_exc()
            return {"error": str(e)}
