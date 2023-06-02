import time
import json
import os
import re
import asyncio

from fastapi import APIRouter, Query, HTTPException
from fastapi.responses import Response, StreamingResponse

from refact_self_hosting import env

from pydantic import BaseModel


__all__ = ["TabFinetuneRouter"]


def sanitize_run_id(run_id: str):
    if not re.fullmatch(r"[0-9a-fA-Z-\.]{2,30}", run_id):
        raise HTTPException(status_code=400, detail="Invalid run id")


async def stream_text_file(ft_path):
    cnt = 0
    f = open(ft_path, "r")
    anything_new_ts = time.time()
    try:
        while True:
            cnt += 1
            line = f.readline()
            if not line:
                print("sleep", f.fileno())
                if anything_new_ts + 120 < time.time():
                    break
                await asyncio.sleep(1)
                continue
            anything_new_ts = time.time()
            yield line
    finally:
        f.close()


class TabFinetuneConfig(BaseModel):
    limit_training_time_minutes: int = Query(default=60, ge=1, le=480)   # 480 minutes is 8 hours
    run_at_night: bool = False
    run_at_night_time: str = Query(default="04:00", regex="([0-9]{1,2}):([0-9]{2})")
    auto_delete_n_runs: int = Query(default=5, ge=2, le=100)


class TabFinetuneRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        super().add_api_route("/tab-finetune-config-and-runs", self._tab_finetune_config_and_runs, methods=["GET"])
        super().add_api_route("/tab-finetune-log/{run_id}", self._tab_funetune_log, methods=["GET"])
        super().add_api_route("/tab-finetune-progress-svg/{run_id}", self._tab_funetune_progress_svg, methods=["GET"])
        super().add_api_route("/tab-finetune-config-save", self._tab_files_config_save, methods=["POST"])

    async def _tab_finetune_config_and_runs(self):
        result = {
            "finetune_runs": [],
            "config": {
                "limit_training_time_minutes": "60",
                "run_at_night": "True",
                "run_at_night_time": "04:00",
                "auto_delete_n_runs": "5",
            },
        }
        for dirname in sorted(os.listdir(env.DIR_LORAS)):
            if not os.path.isdir(os.path.join(env.DIR_LORAS, dirname)):
                continue
            d = {
                "run_id": dirname,
                "worked_minutes": "0",
                "worked_steps": "0",
                "status": "unknown",  # working, starting, completed, failed
            }
            status_fn = os.path.join(ft_path, dirname, "status.json")
            if os.path.exists(status_fn):
                d.update(json.load(open(status_fn, "r")))
            result["finetune_runs"].append(d)
        if os.path.exists(env.CONFIG_FINETUNE):
            result["config"] = json.load(open(env.CONFIG_FINETUNE, "r"))
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_funetune_log(self, run_id: str):
        sanitize_run_id(run_id)
        ft_path = os.path.join(env.DIR_LORAS, run_id, "log.txt")
        return StreamingResponse(
            stream_text_file(ft_path),
            media_type="text/plain",
            headers={
                "Content-Disposition": "attachment; filename=finetune.log",
                "Content-Type": "text/plain",
            },
        )

    async def _tab_funetune_progress_svg(self, run_id: str):
        sanitize_run_id(run_id)
        svg_path = os.path.join(env.DIR_LORAS, run_id, "progress.svg")
        if os.path.exists(svg_path):
            svg = open(svg_path, "r").read()
        else:
            svg = "<svg width=\"432\" height=\"216\" xmlns=\"http://www.w3.org/2000/svg\">"
            svg += '<path d="M 50 10 L 200 150 L 350 200 L 50 200 L 50 10" stroke="#AAA" stroke-width="2" fill="#DDD" />'
            svg += "</svg>"
        return Response(svg, media_type="image/svg+xml")

    async def _tab_files_config_save(self, config: TabFinetuneConfig):
        with open(env.CONFIG_FINETUNE, "w") as f:
            json.dump(config.dict(), f, indent=4)

