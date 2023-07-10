import json
import os
import copy

from fastapi import APIRouter, Request, Query
from fastapi.responses import Response, JSONResponse

from self_hosting_machinery.webgui.selfhost_webutils import log
from known_models_db.refact_known_models import models_mini_db
from self_hosting_machinery import env


from known_models_db.refact_toolbox_db import modelcap_records

from pydantic import BaseModel
from typing import Dict, Set


__all__ = ["TabHostRouter"]


class TabHostModelRec(BaseModel):
    gpus_min: int = Query(default=0, ge=0, le=8)


class TabHostModelsAssign(BaseModel):
    model_assign: Dict[str, TabHostModelRec] = {}
    completion: str
    openai_api_enable: bool = False


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
        if post.completion not in post.model_assign:
            for info in _models()["models"]:
                if info["has_completion"] and info["name"] in post.model_assign:
                    post.completion = info["name"]
                    break
            else:
                post.completion = ""
        with open(env.CONFIG_INFERENCE, "w") as f:
            json.dump(post.dict(), f, indent=4)
        models_to_watchdog_configs()
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
    models_info = []

    def _capabilities(func_type: str) -> Set:
        return {
            capability
            for func in modelcap_records.db
            for capability in func.model
            if func.type == func_type
        }

    chat_caps = _capabilities("chat")
    toolbox_caps = _capabilities("toolbox")
    for k, rec in models_mini_db.items():
        if rec.get("hidden", False):
            continue
        models_info.append({
            "name": k,
            "has_completion": bool("completion" in rec["filter_caps"]),
            "has_finetune": bool("finetune" in rec["filter_caps"]),
            "has_toolbox": bool(toolbox_caps.intersection(rec["filter_caps"])),
            "has_chat": bool(rec["chat_scratchpad_class"]) and bool(chat_caps.intersection(rec["filter_caps"])),
        })
    return {"models": models_info}


def models_to_watchdog_configs():
    gpus = _gpus()["gpus"]
    # models = _models()["models"]
    inference_config = _model_assignment()
    model_assignment = inference_config["model_assign"]
    # This must work or installation is bad
    model_cfg_template = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))
    cursor = 0
    dont_freeze = 0
    while cursor < len(gpus):
        dont_freeze += 1
        if dont_freeze > 100:
            break
        for k, set_gpus in model_assignment.items():
            if k not in models_mini_db.keys():
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

    # Integrations
    integrations = {}
    if os.path.exists(env.CONFIG_INTEGRATIONS):
        integrations = json.load(open(env.CONFIG_INTEGRATIONS, 'r'))

    openai_api_key = integrations.get("openai_api_key", "")
    openai_watchdog_cfg_fn = os.path.join(env.DIR_WATCHDOG_D, "openai_api_worker.cfg")

    if inference_config.get("openai_api_enable", False) and openai_api_key.startswith("sk-"):
        cfg = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "openai_api_worker.cfg"), 'r'))
        cfg.pop('unfinished')
        cfg['command_line'].append('--openai_key')
        cfg['command_line'].append(openai_api_key)
        with open(openai_watchdog_cfg_fn + ".tmp", "w") as f:
            json.dump(cfg, f, indent=4)
        os.rename(openai_watchdog_cfg_fn + ".tmp", openai_watchdog_cfg_fn)
    else:
        try:
            os.unlink(openai_watchdog_cfg_fn)
        except:
            pass


def first_run():
    gpus = _gpus()["gpus"]
    default_config = {
        "model_assign": {
            "CONTRASTcode/3b/multi":  {
                'gpus_min': 1,
                'gpus_max': len(gpus),
            }
        },
        'completion': "CONTRASTcode/3b/multi",
    }
    if not os.path.exists(env.CONFIG_INFERENCE):
        with open(env.CONFIG_INFERENCE, "w") as f:
            json.dump(default_config, f, indent=4)
    models_to_watchdog_configs()
