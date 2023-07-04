import json
import os
import asyncio
import aiohttp
import copy

from fastapi import APIRouter, Request, Query, UploadFile, HTTPException
from fastapi.responses import Response, JSONResponse

from refact_self_hosting.webgui.selfhost_webutils import log
from refact_self_hosting import env
import refact_known_models


from refact_toolbox_db import modelcap_records

from pydantic import BaseModel, Required
from typing import Dict, Optional


__all__ = ["TabHostRouter"]


class TabHostModelRec(BaseModel):
    gpus_min: int = Query(default=0, ge=0, le=8)


class TabHostModelsAssign(BaseModel):
    model_assign: Dict[str, TabHostModelRec] = {}
    openai_enable: bool = False


class TabHostRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/tab-host-have-gpus", self._tab_host_have_gpus, methods=["GET"])
        self.add_api_route("/tab-host-models-get", self._tab_host_models_get, methods=["GET"])
        self.add_api_route("/tab-host-models-assign", self._tab_host_models_assign, methods=["POST"])

    async def _tab_host_have_gpus(self, request: Request):
        return Response(json.dumps(_gpus(include_busy=True), indent=4) + "\n")

    async def _tab_host_models_get(self, request: Request):
        return Response(json.dumps({**_models(), **_model_assignment()}, indent=4) + "\n")

    async def _tab_host_models_assign(self, post: TabHostModelsAssign, request: Request):
        with open(env.CONFIG_INFERENCE, "w") as f:
            json.dump(post.dict(), f, indent=4)
        _update_watchdog_d()
        return JSONResponse("OK")


def _gpus(include_busy: bool = False):
    if os.path.exists(env.CONFIG_ENUM_GPUS):
        j1 = json.load(open(env.CONFIG_ENUM_GPUS, "r"))
    else:
        j2 = {"gpus": []}
    if include_busy:
        j2 = json.load(open(env.CONFIG_BUSY_GPUS, "r"))
        j1len = len(j1["gpus"])
        j2len = len(j2["gpus"])
        for i in range(min(j1len, j2len)):
            j1["gpus"][i].update(j2["gpus"][i])
    return j1


def _model_assignment():
    if os.path.exists(env.CONFIG_INFERENCE):
        j = json.load(open(env.CONFIG_INFERENCE, "r"))
    else:
        j = {"model_assign": {}}
    return j


def _models():
    j = {"models": []}
    for k, rec in refact_known_models.models_mini_db.items():
        if rec.get("hidden", False):
            continue
        j["models"].append({
            "name": k,
            "has_chat": not not rec["chat_scratchpad_class"],
            "has_toolbox": False,
        })
        k_filter_caps = rec["filter_caps"]
        for rec in modelcap_records.db:
            rec_models = rec.model
            if not isinstance(rec_models, list):
                rec_models = [rec_models]
            for test in rec_models:
                if test in k_filter_caps:
                    # print("model %s has toolbox because %s" % (k, rec.function_name))
                    j["models"][-1]["has_toolbox"] = True
                    break
    return j


def _update_watchdog_d():
    gpus = _gpus()["gpus"]
    # models = _models()["models"]
    model_assignment = _model_assignment()["model_assign"]
    # This must work or installation is bad
    model_cfg_template = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))
    cursor = 0
    dont_freeze = 0
    while cursor < len(gpus):
        dont_freeze += 1
        if dont_freeze > 100:
            break
        for k, set_gpus in model_assignment.items():
            if k not in refact_known_models.models_mini_db.keys():
                log("unknown model '%s', skipping" % k)
                continue
            if set_gpus["gpus_min"] > 8:
                log("invalid gpu count %d, skipping" % set_gpus)
                continue
            for g in range(set_gpus["gpus_min"]):
                gpus[cursor]["run-me"] = k
                cursor += 1
                if cursor >= len(gpus):
                    break
            if cursor >= len(gpus):
                break
    for gpu_i, gpu in enumerate(gpus):
        if "run-me" in gpu:
            with open(os.path.join(env.DIR_WATCHDOG_D, "model-gpu%i.cfg" % gpu_i), "w") as f:
                model_cfg_j = copy.deepcopy(model_cfg_template)
                model_cfg_j["command_line"].append("--model")
                model_cfg_j["command_line"].append(gpu["run-me"])
                model_cfg_j["gpus"].append(gpu_i)
                del model_cfg_j["unfinished"]
                json.dump(model_cfg_j, f, indent=4)
        else:
            try:
                os.unlink(os.path.join(env.DIR_WATCHDOG_D, "model-gpu%i.cfg" % gpu_i))
            except:
                pass


def first_run():
    gpus = _gpus()["gpus"]
    default_config = {
        "model_assign": {
            "CONTRASTcode/3b/multi":  {'gpus_min': 1, 'gpus_max': len(gpus)}
        }
    }
    if not os.path.exists(env.CONFIG_INFERENCE):
        with open(env.CONFIG_INFERENCE, "w") as f:
            json.dump(default_config, f, indent=4)
    _update_watchdog_d()
