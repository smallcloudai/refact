import os
import re
import json
import glob
import asyncio

from typing import List, Optional, Iterable
from datetime import datetime

from fastapi import APIRouter
from fastapi.responses import StreamingResponse, Response
from self_hosting_machinery.scripts import env
from self_hosting_machinery.webgui.selfhost_webutils import log


def fn_to_date(fn: str, raise_on_err: bool = False) -> Optional[datetime]:
    try:
        # searching watchdog_20230821.log -> 20230821
        if not (match := re.search(r'_(\d{8})\.', os.path.basename(fn))):
            raise ValueError(f'Failed to parse date')
        date_str = match.group(1)
        # 20230821 -> datetime(2023, 8, 21)
        return datetime.strptime(date_str, "%Y%m%d")
    except Exception as e:
        if raise_on_err:
            raise e
        log(f"Error: {e} parsing date from filename: {fn}")


def remove_outdated_logs(list_of_files: List[str], keep_days) -> None:
    try:
        dt_now = datetime.now()
        # generator: if date is not None and difference in days > keep_days -> remove
        for f in (f for f in list_of_files if (d := fn_to_date(f)) and (dt_now - d).days > keep_days):
            try:
                os.remove(f)
                log(f"Removed outdated log file: {f}")
            except Exception as e:
                log(f"Error: {e} removing outdated log file: {f}")
    except Exception as e:
        log(f"Error: {e} removing outdated logs")


def get_log_files():
    log_files = glob.glob(f'{env.DIR_LOGS}/watchdog_*.log')
    # it might be better to sort by actual date, but it's unnecessary
    return sorted(log_files, reverse=True)


class TabServerLogRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-server-log-get", self._tab_server_log_get, methods=["GET"])
        self.add_api_route("/tab-server-log-plain/{log_name}", self._tab_server_log_plain, methods=["GET"])

    async def _tab_server_log_get(self):
        remove_outdated_logs(get_log_files(), keep_days=7)
        results: List[str] = [os.path.basename(f) for f in get_log_files()]
        latest_log: str = results[0] if results else ""
        return Response(json.dumps({
            "all_logs": results,
            "latest_log": latest_log,
        }, indent=4), media_type="application/json")

    async def _tab_server_log_plain(self, log_name: str = "", stream: bool = False):
        async def stream_log():
            with open(right_file, "r", encoding="utf-8") as f:
                while True:
                    tmp = f.read()
                    if not stream and not tmp:
                        break
                    yield tmp
                    await asyncio.sleep(0.5)

        list_of_files = get_log_files()
        if log_name in ["", "latest"]:
            list_of_files = list_of_files[:1]
        else:
            list_of_files = [f for f in list_of_files if f.endswith(log_name)]
        if len(list_of_files) == 0:
            return Response("File \"%s\" not found\n" % log_name, media_type="text/plain")
        right_file = list_of_files[0]

        return StreamingResponse(
            stream_log(),
            media_type="text/event-stream"
        )
