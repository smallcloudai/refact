import os
import json
import glob
import asyncio

from fastapi import APIRouter
from fastapi.responses import StreamingResponse, Response
from self_hosting_machinery.scripts import env  # REFACTORME


class TabServerLogRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-server-log-get", self._tab_server_log_get, methods=["GET"])
        self.add_api_route("/tab-server-log-plain/{log_name}", self._tab_server_log_plain, methods=["GET"])

    async def _tab_server_log_get(self):
        list_of_files = glob.glob(f'{env.DIR_LOGS}/watchdog_*.log')
        list_of_files.sort()
        results = []
        latest_log = ""
        for f in list_of_files:
            f = os.path.basename(f)
            results.append(f)
            latest_log = f
        return Response(json.dumps({
            "all_logs": results,
            "latest_log": latest_log,
        }, indent=4), media_type="application/json")

    async def _tab_server_log_plain(self, log_name: str = "", stream: bool = False):
        async def get_log(right_file):
            with open(right_file, "r", encoding="utf-8") as f:
                while True:
                    tmp = f.read()
                    if not stream and not tmp:
                        break
                    yield tmp
                    await asyncio.sleep(0.5)

        list_of_files = glob.glob(f'{env.DIR_LOGS}/watchdog_*.log')
        if log_name in ["", "latest"]:
            list_of_files.sort()
            list_of_files = list_of_files[-1:]
        else:
            list_of_files = [f for f in list_of_files if f.endswith(log_name)]
        if len(list_of_files) == 0:
            return Response("File \"%s\" not found\n" % log_name, media_type="text/plain")
        right_file = list_of_files[-1]

        return StreamingResponse(
            get_log(right_file),
            media_type="text/event-stream"
        )
