import shutil

import time
import json
import os
import re
import asyncio
from typing import Optional

from fastapi import APIRouter, Query, HTTPException
from fastapi.responses import Response, StreamingResponse, JSONResponse

from self_hosting_machinery.scripts import best_lora
from refact_data_pipeline.finetune.finetune_utils import get_active_loras
from refact_data_pipeline.finetune.finetune_utils import get_finetune_config
from refact_data_pipeline.finetune.finetune_filtering_defaults import finetune_filtering_defaults
from refact_data_pipeline.finetune.finetune_train_defaults import finetune_train_defaults
from self_hosting_machinery import env

from pydantic import BaseModel


__all__ = ["TabFinetuneRouter", "get_finetune_runs"]


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
    run_at_night: bool = False
    run_at_night_time: str = Query(default="04:00", regex="([0-9]{1,2}):([0-9]{2})")
    auto_delete_n_runs: int = Query(default=5, ge=2, le=100)


class TabFinetuneActivate(BaseModel):
    model: str
    lora_mode: str = Query(default="default", regex="off|latest-best|specific")
    specific_lora_run_id: str = Query(default="")
    specific_checkpoint: str = Query(default="")


class FilteringSetup(BaseModel):
    filter_loss_threshold: Optional[float] = Query(default=None, gt=2, le=10)
    # limit_train_files: Optional[int] = Query(default=None, gt=20, le=100000)
    # limit_test_files: Optional[int] = Query(default=None, gt=1, le=5)
    # filter_gradcosine_threshold: Optional[float] = Query(default=None, gt=-1.0, le=0.5)
    # limit_time_seconds: Optional[int] = Query(default=None, gt=300, le=3600*12)
    # use_gpus_n: Optional[int] = Query(default=False, gt=1, le=8)
    # low_gpu_mem_mode: Optional[bool] = Query(default=True)


class TabFinetuneTrainingSetup(BaseModel):
    limit_time_seconds: Optional[int] = Query(default=600, ge=600, le=3600*48)
    lr: Optional[float] = Query(default=30e-5, ge=1e-5, le=300e-5)
    batch_size: Optional[int] = Query(default=128, ge=4, le=1024)
    warmup_num_steps: Optional[int] = Query(default=10, ge=1, le=100)
    weight_decay: Optional[float] = Query(default=0.1, ge=0.0, le=1.0)
    use_heuristics: Optional[bool] = Query(default=True)
    train_steps: Optional[int] = Query(default=250, ge=10, le=5000)
    lr_decay_steps: Optional[int] = Query(default=250, ge=10, le=5000)
    lora_r: Optional[int] = Query(default=16, ge=4, le=128)
    lora_alpha: Optional[float] = Query(default=32, ge=1, le=256)
    lora_init_scale: Optional[float] = Query(default=0.01, ge=0.0, le=1.0)
    lora_dropout: Optional[float] = Query(default=0.01, ge=0.0, le=0.5)
    low_gpu_mem_mode: Optional[bool] = Query(default=True)


def get_finetune_runs():
    res = []
    anyone_works = False
    if not os.path.isdir(env.DIR_LORAS):
        return [], anyone_works
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
        try:
            d["model_name"] = best_lora.get_run_model_name(dir_path)
        except RuntimeError:
            continue
        status_fn = os.path.join(dir_path, "status.json")
        if os.path.exists(status_fn):
            d.update(json.load(open(status_fn, "r")))
        if d["status"] in ["working", "starting"]:
            mtime = os.path.getmtime(status_fn)
            if mtime + 300 < time.time():
                d["status"] = "failed"
            else:
                anyone_works = True
        d["checkpoints"] = []
        checkpoints_dir = os.path.join(dir_path, "checkpoints")
        if os.path.isdir(checkpoints_dir):
            for checkpoint_dir in sorted(os.listdir(checkpoints_dir)):
                checkpoint_path = os.path.join(checkpoints_dir, checkpoint_dir)
                if not os.path.isdir(checkpoint_path):
                    continue
                d["checkpoints"].append({
                    "checkpoint_name": checkpoint_dir,
                })
        res.append(d)
    return res, anyone_works


class TabFinetuneRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-finetune-config-and-runs", self._tab_finetune_config_and_runs, methods=["GET"])
        self.add_api_route("/tab-finetune-log/{run_id}", self._tab_funetune_log, methods=["GET"])
        self.add_api_route("/tab-finetune-progress-svg/{run_id}", self._tab_funetune_progress_svg, methods=["GET"])
        self.add_api_route("/tab-finetune-schedule-save", self._tab_finetune_schedule_save, methods=["POST"])
        self.add_api_route("/tab-finetune-activate", self._tab_finetune_activate, methods=["POST"])
        self.add_api_route("/tab-finetune-run-now", self._tab_finetune_run_now, methods=["GET"])
        self.add_api_route("/tab-finetune-stop-now", self._tab_finetune_stop_now, methods=["GET"])
        self.add_api_route("/tab-finetune-remove/{run_id}", self._tab_finetune_remove, methods=["GET"])
        self.add_api_route("/tab-finetune-smart-filter-setup", self._tab_finetune_smart_filter_setup, methods=["POST"])
        self.add_api_route("/tab-finetune-smart-filter-get", self._tab_finetune_smart_filter_get, methods=["GET"])
        self.add_api_route("/tab-finetune-training-setup", self._tab_finetune_training_setup, methods=["POST"])
        self.add_api_route("/tab-finetune-training-get", self._tab_finetune_training_get, methods=["GET"])
        self.__setup_filter_status()

    async def _tab_finetune_config_and_runs(self):
        runs, anyone_works = get_finetune_runs()
        result = {
            "finetune_runs": runs,
            "config": {
                "limit_training_time_minutes": "60",
                "run_at_night": "True",
                "run_at_night_time": "04:00",
                "auto_delete_n_runs": "5",
                **get_finetune_config(),  # TODO: why we mix finetune config for training and schedule?
            },
            "filtering_status": "unknown",
            "finetune_working_now": anyone_works,
            "active": get_active_loras(),
        }
        if os.path.exists(env.CONFIG_FINETUNE_FILTER_STATS):
            c = json.load(open(env.CONFIG_FINETUNE_FILTER_STATS, "r"))
            result["filtering_status"] = c['status']
        result["finetune_latest_best"] = best_lora.find_best_lora(result["config"]["model_name"])
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_finetune_smart_filter_setup(self, post: FilteringSetup):
        validated = post.dict()
        for dkey, dval in finetune_filtering_defaults.items():
            if dkey in validated and (validated[dkey] == dval or validated[dkey] is None):
                del validated[dkey]
        with open(env.CONFIG_HOW_TO_FILTER + ".tmp", "w") as f:
            json.dump(post.dict(), f, indent=4)
        os.rename(env.CONFIG_HOW_TO_FILTER + ".tmp", env.CONFIG_HOW_TO_FILTER)
        return JSONResponse("OK")

    async def _tab_finetune_smart_filter_get(self):
        result = {
            "defaults": finetune_filtering_defaults,
            "user_config": {}
        }
        if os.path.exists(env.CONFIG_HOW_TO_FILTER):
            result["user_config"] = json.load(open(env.CONFIG_HOW_TO_FILTER))
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_finetune_training_setup(self, post: TabFinetuneTrainingSetup):
        validated = post.dict()
        for dkey, dval in finetune_train_defaults.items():
            if dkey in validated and (validated[dkey] == dval or validated[dkey] is None):
                del validated[dkey]
        with open(env.CONFIG_FINETUNE + ".tmp", "w") as f:
            json.dump(post.dict(), f, indent=4)
        os.rename(env.CONFIG_FINETUNE + ".tmp", env.CONFIG_FINETUNE)
        return JSONResponse("OK")

    async def _tab_finetune_training_get(self):
        result = {
            "defaults": finetune_train_defaults,
            "user_config": get_finetune_config(),
        }
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_funetune_log(self, run_id: str):
        sanitize_run_id(run_id)
        log_path = os.path.join(env.DIR_LORAS, run_id, "log.txt")
        if not os.path.isfile(log_path):
            return Response("File '%s' not found" % log_path, status_code=404)
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
            svg = '<svg width="432" height="217" viewBox="0 0 432 217" fill="none" xmlns="http://www.w3.org/2000/svg">'
            svg += '<line x1="50" y1="10.496" x2="350" y2="10.496" stroke="#EFEFEF"/>'
            svg += '<line x1="50" y1="200.496" x2="350" y2="200.496" stroke="#EFEFEF"/>'
            svg += '<line x1="50" y1="162.496" x2="350" y2="162.496" stroke="#EFEFEF"/>'
            svg += '<line x1="50" y1="124.496" x2="350" y2="124.496" stroke="#EFEFEF"/>'
            svg += '<line x1="50" y1="86.496" x2="350" y2="86.496" stroke="#EFEFEF"/>'
            svg += '<line x1="50" y1="48.496" x2="350" y2="48.496" stroke="#EFEFEF"/>'
            svg += '<path d="M50 10.996L140 110.996L200.98 89.6939L350 200.996" stroke="#CDCDCD" stroke-width="2"/>'
            svg += '</svg>'
        return Response(svg, media_type="image/svg+xml")

    async def _tab_finetune_schedule_save(self, config: TabFinetuneConfig):
        pass

    def __setup_filter_status(self):
        if os.path.isfile(env.FLAG_LAUNCH_FINETUNE_FILTER_ONLY):
            if os.path.isfile(env.CONFIG_FINETUNE_FILTER_STATS):
                config = json.load(open(env.CONFIG_FINETUNE_FILTER_STATS))
            else:
                config = {}
            config['status'] = 'starting'
            json.dump(config, open(env.CONFIG_FINETUNE_FILTER_STATS, 'w'), indent=4)

    async def _tab_finetune_run_now(self, filter_only: bool = False):
        flag = env.FLAG_LAUNCH_FINETUNE_FILTER_ONLY if filter_only else env.FLAG_LAUNCH_FINETUNE
        with open(flag, "w") as f:
            f.write("")
        if filter_only:
            self.__setup_filter_status()
        return JSONResponse("OK")

    async def _tab_finetune_stop_now(self):
        with open(env.FLAG_STOP_FINETUNE, "w") as f:
            f.write("")
        return JSONResponse("OK")

    async def _tab_finetune_remove(self, run_id: str):
        sanitize_run_id(run_id)
        home_path = os.path.join(env.DIR_LORAS, run_id)
        if not os.path.exists(home_path):
            return Response("Run id '%s' not found" % home_path, status_code=404)
        shutil.rmtree(home_path)
        return JSONResponse("OK")

    async def _tab_finetune_activate(self, activate: TabFinetuneActivate):
        with open(env.CONFIG_ACTIVE_LORA, "w") as f:
            f.write(activate.json(indent=4))
        return JSONResponse("OK")
