import time
import json
import os
import re
import asyncio

from fastapi import APIRouter, Query, HTTPException
from fastapi.responses import Response, StreamingResponse, JSONResponse

from refact_self_hosting import env
from refact_self_hosting.webgui.selfhost_webutils import log

from pydantic import BaseModel


__all__ = ["TabFinetuneRouter"]


def sanitize_run_id(run_id: str):
    if not re.fullmatch(r"[0-9a-zA-Z-\.]{2,40}", run_id):
        raise HTTPException(status_code=400, detail="Invalid run id \"%s\"" % run_id)


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


class TabFinetuneActivate(BaseModel):
    model: str
    lora_run_id: str   # specific or "latest"
    checkpoint: str    # specific or "best"


class TabFinetuneRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-finetune-config-and-runs", self._tab_finetune_config_and_runs, methods=["GET"])
        self.add_api_route("/tab-finetune-log/{run_id}", self._tab_funetune_log, methods=["GET"])
        self.add_api_route("/tab-finetune-progress-svg/{run_id}", self._tab_funetune_progress_svg, methods=["GET"])
        self.add_api_route("/tab-finetune-config-save", self._tab_finetune_config_save, methods=["POST"])
        self.add_api_route("/tab-finetune-activate", self._tab_finetune_activate, methods=["POST"])
        self.add_api_route("/tab-finetune-run-now", self._tab_finetune_run_now, methods=["GET"])

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
            dir_path = os.path.join(env.DIR_LORAS, dirname)
            if not os.path.isdir(dir_path):
                continue
            d = {
                "run_id": dirname,
                "worked_minutes": "0",
                "worked_steps": "0",
                "status": "unknown",  # working, starting, completed, failed
            }
            status_fn = os.path.join(dir_path, "status.json")
            if os.path.exists(status_fn):
                d.update(json.load(open(status_fn, "r")))
            if d["status"] in ["working", "starting"]:
                mtime = os.path.getmtime(status_fn)
                if mtime + 600 < time.time():
                    d["status"] = "failed"
            d["checkpoints"] = []
            checkpoints_dir = os.path.join(dir_path, "checkpoints")
            print(checkpoints_dir)
            if os.path.isdir(checkpoints_dir):
                for checkpoint_dir in sorted(os.listdir(checkpoints_dir)):
                    print(checkpoint_dir)
                    checkpoint_path = os.path.join(checkpoints_dir, checkpoint_dir)
                    if not os.path.isdir(checkpoint_path):
                        continue
                    d["checkpoints"].append({
                        "checkpoint_name": checkpoint_dir,
                        })
            result["finetune_runs"].append(d)
        if os.path.exists(env.CONFIG_FINETUNE):
            result["config"] = json.load(open(env.CONFIG_FINETUNE, "r"))
        if os.path.exists(env.CONFIG_ACTIVE_LORA):
            result["active"] = json.load(open(env.CONFIG_ACTIVE_LORA, "r"))
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_funetune_log(self, run_id: str):
        sanitize_run_id(run_id)
        log_path = os.path.join(env.DIR_LORAS, run_id, "log.txt")
        return StreamingResponse(
            stream_text_file(log_path),
            media_type="text/event-stream"
        )

    async def _tab_funetune_progress_svg(self, run_id: str):
        sanitize_run_id(run_id)
        svg_path = os.path.join(env.DIR_LORAS, run_id, "progress.svg")
        if os.path.exists(svg_path):
            svg = open(svg_path, "r").read()
        else:
            svg = "<svg width=\"432\" height=\"216\" xmlns=\"http://www.w3.org/2000/svg\">"
            svg += '<path d="M 50 10 L 140 110 L 350 200 L 50 200 L 50 10" stroke="#AAA" stroke-width="2" fill="#DDD" />'
            svg += "</svg>"
        return Response(svg, media_type="image/svg+xml")

    async def _tab_finetune_config_save(self, config: TabFinetuneConfig):
        with open(env.CONFIG_FINETUNE, "w") as f:
            json.dump(config.dict(), f, indent=4)

    async def _tab_finetune_run_now(self):
        with open(env.FLAG_LAUNCH_FINETUNE, "w") as f:
            f.write("1")
        return JSONResponse("OK")

    async def _tab_finetune_activate(self, activate: TabFinetuneActivate):
        with open(env.CONFIG_ACTIVE_LORA, "w") as f:
            f.write(activate.json())
        return JSONResponse("OK")
