import os

import glob

import time

from fastapi import APIRouter
from fastapi.responses import StreamingResponse

from refact_self_hosting.env import DIR_LOGS


class TabServerLogRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-server-log-get-log", self._tab_server_log_get_log, methods=["GET"])

    async def _tab_server_log_get_log(self):
        def get_log():
            list_of_files = glob.glob(f'{DIR_LOGS}/*')
            latest_file = max(list_of_files, key=os.path.getctime)

            with open(latest_file, "r", encoding="utf-8") as f:
                yield f.read()
                while True:
                    yield f.read()
                    time.sleep(0.5)

        return StreamingResponse(
            get_log(),
            media_type="text/event-stream"
        )
