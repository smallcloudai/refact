import asyncio
import os
import json
import glob
from typing import AsyncIterator

from fastapi import APIRouter
from fastapi.responses import StreamingResponse, Response
from refact_utils.scripts import env


async def tail(file_path: str, last_n_lines: int, stream: bool) -> AsyncIterator[str]:
    if stream:
        cmd = ["tail", "-f", "-n", str(last_n_lines), file_path]
    else:
        cmd = ["tail", f"-n {last_n_lines}", file_path]

    print(f"EXEC: {' '.join(cmd)}")
    process = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await process.communicate()

    if process.returncode == 0:
        while True:
            output = await process.stdout.readline()
            if not output:
                if not stream:
                    break
                await asyncio.sleep(0.1)
                continue
            yield output.decode()

    else:
        raise Exception(f"{' '.join(cmd)} failed: {stderr.decode()}")


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
        list_of_files = glob.glob(f'{env.DIR_LOGS}/watchdog_*.log')
        if log_name in ["", "latest"]:
            list_of_files.sort()
            list_of_files = list_of_files[-1:]
        else:
            list_of_files = [f for f in list_of_files if f.endswith(log_name)]
        if len(list_of_files) == 0:
            return Response("File \"%s\" not found\n" % log_name, media_type="text/plain")
        right_file = list_of_files[-1]

        async def streamer(last_n_lines: int = 10_000) -> AsyncIterator[str]:
            async for line in tail(right_file, last_n_lines, stream):
                yield line

        return StreamingResponse(
            streamer(),
            media_type="text/event-stream"
        )
