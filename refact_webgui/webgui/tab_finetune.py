import shutil

import time
import json
import os
import re
import asyncio

from fastapi import APIRouter, Request, Header, HTTPException, Query
from fastapi.responses import Response, StreamingResponse, JSONResponse

from refact_utils.scripts import env
from refact_utils.scripts import best_lora
from refact_utils.finetune.utils import running_models_and_loras
from refact_utils.finetune.utils import get_finetune_config
from refact_utils.finetune.utils import get_finetune_filter_stat
from refact_utils.finetune.utils import get_prog_and_status_for_ui
from refact_utils.finetune.utils import get_finetune_runs
from refact_utils.finetune.filtering_defaults import finetune_filtering_defaults
from refact_utils.finetune.train_defaults import finetune_train_defaults
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from pydantic import BaseModel, validator, Field, Required

from typing import Optional, List


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
                if anything_new_ts + 600 < time.time():
                    break
                await asyncio.sleep(1)
                continue
            anything_new_ts = time.time()
            yield line
    finally:
        f.close()


# class TabFinetuneConfig(BaseModel):
#     run_at_night: bool = False
#     run_at_night_time: str = Query(default="04:00", regex="([0-9]{1,2}):([0-9]{2})")
#     auto_delete_n_runs: int = Query(default=5, ge=2, le=100)


class FilteringSetup(BaseModel):
    autoselect_test_files_num: Optional[int] = Query(default=3, gt=1, le=1000)
    filter_loss_threshold: Optional[float] = Query(default=3.0, gt=1.0, le=10.0)
    debug: bool = False
    # use_gpus_n: Optional[int] = Query(default=1, gt=1, le=8)


class TabFinetuneTrainingSetup(BaseModel):
    run_id: Optional[str] = Query(default=Required, regex="^[0-9a-zA-Z-\.\-]{1,30}$")
    model_name: Optional[str] = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")
    # limit_time_seconds: Optional[int] = Query(default=600, ge=600, le=3600*48)
    trainable_embeddings: Optional[bool] = Query(default=False)
    low_gpu_mem_mode: Optional[bool] = Query(default=True)
    lr: Optional[float] = Query(default=30e-5, ge=1e-5, le=300e-5)
    batch_size: Optional[int] = Query(default=128, ge=4, le=1024)
    warmup_num_steps: Optional[int] = Query(default=10, ge=1, le=100)
    weight_decay: Optional[float] = Query(default=0.1, ge=0.0, le=1.0)
    # use_heuristics: Optional[bool] = Query(default=True)
    train_steps: Optional[int] = Query(default=250, ge=10, le=5000)
    lr_decay_steps: Optional[int] = Query(default=250, ge=10, le=5000)
    lora_r: Optional[int] = Query(default=16, ge=4, le=64)
    lora_alpha: Optional[float] = Query(default=32, ge=4, le=128)
    lora_dropout: Optional[float] = Query(default=0.01, ge=0.0, le=0.5)
    # gpus: Optional[List[int]] = Query(default=[0], ge=0, le=8)
    gpus: List[int] = Field(..., example=[0], ge=0, le=8)


class RenamePost(BaseModel):
    run_id_old: str
    run_id_new: str

    @validator('run_id_new')
    def validate_run_ids(cls, v, values) -> str:
        if not v.strip():
            raise HTTPException(status_code=400, detail="must be non-empty")
        if len(v) >= 30:
            raise HTTPException(status_code=400, detail="must be less than 30 characters")
        if not re.match("^[a-zA-Z0-9_-]*$", v):
            raise HTTPException(status_code=400, detail="must contain only Latin alphabet, numbers, spaces, and underscores")
        if 'run_id_old' in values and v == values['run_id_old']:
            raise HTTPException(status_code=400, detail="run_id_new is similar to run_id_old")
        return v


class TabFinetuneRouter(APIRouter):

    def __init__(self, model_assigner: ModelAssigner, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/running-models-and-loras", self._running_models_and_loras, methods=["GET"])
        self.add_api_route("/tab-finetune-rename", self._tab_finetune_rename, methods=["POST"])
        self.add_api_route("/tab-finetune-get", self._tab_finetune_get, methods=["GET"])
        self.add_api_route("/tab-finetune-config-and-runs", self._tab_finetune_config_and_runs, methods=["GET"])
        self.add_api_route("/tab-finetune-log/{run_id}", self._tab_funetune_log, methods=["GET"])
        self.add_api_route("/tab-finetune-filter-log", self._tab_finetune_filter_log, methods=["GET"])
        self.add_api_route("/tab-finetune-progress-svg/{run_id}", self._tab_funetune_progress_svg, methods=["GET"])
        # self.add_api_route("/tab-finetune-schedule-save", self._tab_finetune_schedule_save, methods=["POST"])
        # self.add_api_route("/tab-finetune-run-now", self._tab_finetune_run_now, methods=["GET"])
        # self.add_api_route("/tab-finetune-stop-now", self._tab_finetune_stop_now, methods=["GET"])
        self.add_api_route("/tab-finetune-remove/{run_id}", self._tab_finetune_remove, methods=["GET"])
        self.add_api_route("/tab-finetune-smart-filter-setup", self._tab_finetune_smart_filter_setup, methods=["POST"])
        self.add_api_route("/tab-finetune-smart-filter-get", self._tab_finetune_smart_filter_get, methods=["GET"])
        self.add_api_route("/tab-finetune-training-setup", self._tab_finetune_training_setup, methods=["POST"])
        self.add_api_route("/tab-finetune-training-get", self._tab_finetune_training_get, methods=["GET"])
        self._model_assigner = model_assigner

    async def _running_models_and_loras(self):
        return Response(json.dumps(running_models_and_loras(self._model_assigner), indent=4) + "\n")

    async def _tab_finetune_rename(self, post: RenamePost):
        running = running_models_and_loras(self._model_assigner)
        active_loras = {[*i.split(":"), None, None][:3][1] for v in running.values() for i in v}
        active_loras = {i for i in active_loras if i}
        if post.run_id_old in active_loras:
            raise HTTPException(status_code=400, detail=f"cannot rename {post.run_id_old}: currently in use")

        path_old = os.path.join(env.DIR_LORAS, post.run_id_old)
        path_new = os.path.join(env.DIR_LORAS, post.run_id_new)

        run_config = {
            "status": "preparing",
        }
        if os.path.exists(status_fn := os.path.join(path_old, "status.json")):
            with open(status_fn, "r") as f:
                run_config.update(json.load(f))
        if run_config["status"] not in ["finished", "interrupted", "failed"]:
            raise HTTPException(status_code=400, detail=f"cannot rename {post.run_id_old}: finetune is not finished")

        try:
            os.rename(path_old, path_new)
        except Exception as e:
            raise HTTPException(status_code=400, detail=f"cannot rename {post.run_id_old}: {str(e)}")

        return JSONResponse("OK")

    async def _tab_finetune_get(self):
        prog, status = get_prog_and_status_for_ui()
        working = status in ["starting", "working"]
        result = {
            "prog_name": prog,
            "prog_status": status,
            "filter_working_now": (prog == "prog_filter" and working),
            "finetune_working_now": (prog == "prog_ftune" and working),
            "finetune_filter_stats": {
                **get_finetune_filter_stat(),
            },
            "sources_ready": await self._tab_finetune_get_sources_status(),
        }
        return Response(json.dumps(result, indent=4) + "\n")

    async def _tab_finetune_get_sources_status(self):
        try:
            with open(env.CONFIG_PROCESSING_STATS, "r") as file:
                scan_stats = json.load(file)
                scan_stats_status = scan_stats.get("scan_finished")
                return scan_stats_status
        except FileNotFoundError:
            return False
        except json.JSONDecodeError:
            return False
        except Exception as e:
            return f"Error: {str(e)}"

    async def _tab_finetune_config_and_runs(self):
        runs = get_finetune_runs()
        for run in runs:
            try:
                run["best_checkpoint"] = best_lora.find_best_checkpoint(run["run_id"])
            except Exception as e:
                run["best_checkpoint"] = {"error": str(e)}
        config = get_finetune_config(self._model_assigner.models_db)
        result = {
            "finetune_runs": runs,
            "config": config,
            #     "limit_training_time_minutes": "60",
            #     "run_at_night": "True",
            #     "run_at_night_time": "04:00",
            #     "auto_delete_n_runs": "5",
            #     **config,  # TODO: why we mix finetune config for training and schedule?
            # },
        }
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
        # {
        #     "run_id": "xxxx-20240315-090039",
        #     "model_name": "deepseek-coder/1.3b/base",
        #     "trainable_embeddings": false,
        #     "low_gpu_mem_mode": true,
        #     "lr": "0.0003",
        #     "batch_size": "128",
        #     "warmup_num_steps": "20",
        #     "weight_decay": "0.1",
        #     "train_steps": "250",
        #     "lr_decay_steps": "250",
        #     "lora_r": "16",
        #     "lora_alpha": "32",
        #     "lora_dropout": "0.01",
        #     "gpus": [0, 1, 2]
        # }
        validated = post.dict()
        run_id = post.run_id
        for dkey, dval in finetune_train_defaults.items():
            if dkey in validated and (validated[dkey] == dval or validated[dkey] is None):
                del validated[dkey]

        # values will be used to fill form for the next run
        with open(env.CONFIG_FINETUNE + ".tmp", "w") as f:
            json.dump(validated, f, indent=4)
        os.rename(env.CONFIG_FINETUNE + ".tmp", env.CONFIG_FINETUNE)
        # {
        #     "policy": ["single_shot"],
        #     "interrupt_when_file_appears": "%%",
        #     "save_status": "%%",
        #     "save_status_nickname": "prog_ftune",
        #     "command_line": ["python", "-m", "self_hosting_machinery.finetune.scripts.finetune_sequence"],
        #     "gpus": []
        # }
        ftune_cfg_j = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "filetune.cfg")))
        fn = os.path.join(env.DIR_WATCHDOG_D, "ftune-%s.cfg" % run_id)
        os.make_dirs(os.path.join(env.DIR_LORAS, run_id), exist_ok=False)
        ftune_cfg_j["gpus"] = post.gpus
        ftune_cfg_j["interrupt_when_file_appears"] = os.path.join(env.DIR_LORAS, run_id, "stop.flag")
        ftune_cfg_j["save_status"] = os.path.join(env.DIR_LORAS, run_id, "watchdog_status.out")
        ftune_cfg_j["save_status_nickname"] = run_id
        del ftune_cfg_j["unfinished"]
        for k in validated:
            if k == "gpus":
                continue
            ftune_cfg_j["command_line"].append("--" + k)
            ftune_cfg_j["command_line"].append(str(validated[k]))
        with open(fn + ".tmp", "w") as f:
            json.dump(ftune_cfg_j, f, indent=4)
        os.rename(fn + ".tmp", fn)
        return JSONResponse("OK")

    # async def _tab_finetune_run_now(self, filter_only: bool = False):
    #     flag = env.FLAG_LAUNCH_FINETUNE_FILTER_ONLY if filter_only else env.FLAG_LAUNCH_FINETUNE
    #     with open(flag, "w") as f:
    #         f.write("")
    #     return JSONResponse("OK")

    # async def _tab_finetune_stop_now(self):
    #     # TODO: add run_id to POST, delete cfg
    #     with open(env.FLAG_STOP_FINETUNE, "w") as f:
    #         f.write("")
    #     return JSONResponse("OK")

    async def _tab_finetune_training_get(self):
        result = {
            "defaults": finetune_train_defaults,
            "user_config": get_finetune_config(self._model_assigner.models_db),
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

    async def _tab_finetune_filter_log(self, accepted_or_rejected: str):
        if accepted_or_rejected == "accepted":
            fn = env.LOG_FILES_ACCEPTED_FTF
        else:
            fn = env.LOG_FILES_REJECTED_FTF
        if os.path.isfile(fn):
            return Response(
                open(fn, "r").read(),
                media_type="text/plain"
            )
        else:
            return Response("File list empty\n", media_type="text/plain")

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

    # async def _tab_finetune_schedule_save(self, config: TabFinetuneConfig):
    #     pass

    async def _tab_finetune_remove(self, run_id: str):
        sanitize_run_id(run_id)
        home_path = os.path.join(env.DIR_LORAS, run_id)
        if not os.path.exists(home_path):
            return Response("Run id '%s' not found" % home_path, status_code=404)
        shutil.rmtree(home_path)
        return JSONResponse("OK")
