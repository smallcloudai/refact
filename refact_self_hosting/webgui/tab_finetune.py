import time, json, termcolor, os
import asyncio
from fastapi import APIRouter, Request, Query, Header
from fastapi.responses import Response
from pydantic import BaseModel, Required
from refact_self_hosting.webgui import selfhost_req_queue
from refact_self_hosting.webgui.selfhost_webutils import log
from typing import Dict, List, Optional, Any


router = APIRouter()


@router.get("/tab-finetune-config-and-runs")
async def tab_finetune_config_and_runs(request: Request):
    uploaded_path = os.path.expanduser("~/perm-storage/finetune")
    result = {
        "finetune_runs": [],
        "config": {
            "limit_training_time_minutes": "60",
            "run_at_night": "True",
            "run_at_night_time": "04:00",
            "auto_delete_n_runs": "5",
        },
    }
    for dirname in os.listdir(uploaded_path):
        if not os.path.isdir(os.path.join(uploaded_path, dirname)):
            continue
        result["finetune_runs"].append({
            "run_id": dirname,
            "worked_minutes": "480",
            "worked_steps": "1337",
        })
    cfg_fn = os.path.expanduser("~/perm-storage/tab-finetune.cfg")
    if os.path.exists(cfg_fn):
        result["config"] = json.load(open(cfg_fn, "r"))
    return Response(json.dumps(result, indent=4) + "\n")


@router.get("/tab-finetune-log/{run_id}")
async def tab_funetune_log(request: Request, run_id: str):
    result = {
        "log": ["Line1", "Line2", "Line3", "It was run \"%s\"" % run_id],
    }
    return Response(json.dumps(result, indent=4) + "\n")


@router.get("/tab-finetune-progress-svg/{run_id}")
async def tab_funetune_progress_svg(request: Request, run_id: str):
    svg = "<svg width=\"100%\" height=\"100%\" viewBox=\"0 0 100 100\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">\n"
    svg += "<circle cx=\"50\" cy=\"50\" r=\"40\" stroke=\"black\" stroke-width=\"2\" fill=\"white\" />\n"
    svg += "</svg>"
    return Response(svg + "\n", media_type="image/svg+xml")


class TabFinetuneConfig(BaseModel):
    limit_training_time_minutes: int = Query(default=60, ge=1, le=480)   # 480 minutes is 8 hours
    run_at_night: bool = False
    run_at_night_time: str = Query(default="04:00", regex="([0-9]{1,2}):([0-9]{2})")
    auto_delete_n_runs: int = Query(default=5, ge=2, le=100)


@router.post("/tab-finetune-config-save")
async def tab_files_config_save(config: TabFinetuneConfig, request: Request):
    cfg_fn = os.path.expanduser("~/perm-storage/tab-finetune.cfg")
    with open(cfg_fn, "w") as f:
        json.dump(config.dict(), f, indent=4)

