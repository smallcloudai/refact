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
    gpus_shard: int = Query(default=1, ge=1, le=4)


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
        validated = post.dict()
        current_completion_model = validated.get("completion", "")
        if not current_completion_model or current_completion_model not in post.model_assign:
            for info in _models()["models"]:
                if info["has_completion"] and info["name"] in post.model_assign:
                    validated["completion"] = info["name"]
                    break
            else:
                validated["completion"] = ""
        models_to_watchdog_configs(validated)
        return JSONResponse("OK")


def _gpus(include_busy: bool = False):
    if os.path.exists(env.CONFIG_ENUM_GPUS):
        j1 = json.load(open(env.CONFIG_ENUM_GPUS, "r"))
    else:
        j1 = {"gpus": []}
    if include_busy and os.path.exists(env.CONFIG_BUSY_GPUS):
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


def models_to_watchdog_configs(inference_config):
    gpus = _gpus()["gpus"]
    model_assignment = inference_config["model_assign"]
    # This must work or installation is bad
    model_cfg_template = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))
    cursor = 0
    allowed_to_exist = []
    more_models_than_gpus = False
    for k, assrec in model_assignment.items():
        if k not in models_mini_db.keys():
            log("unknown model '%s', skipping" % k)
            continue
        if assrec["gpus_shard"] not in [1, 2, 4]:
            log("invalid shard count %d, skipping %s" % (assrec["gpus_shard"], k))
            continue
        log("assign model '%s', cursor %d, gpus_shard %d" % (k, cursor, assrec["gpus_shard"]))
        if cursor + assrec["gpus_shard"] > len(gpus):
            more_models_than_gpus = True
            break
        cfg_out = "model-%s.cfg" % k.lower().replace("/", "-")
        allowed_to_exist.append(cfg_out)
        with open(os.path.join(env.DIR_WATCHDOG_D, cfg_out), "w") as f:
            model_cfg_j = copy.deepcopy(model_cfg_template)
            model_cfg_j["command_line"].append("--model")
            model_cfg_j["command_line"].append(k)
            model_cfg_j["gpus"] = list(range(cursor, cursor + assrec["gpus_shard"]))
            del model_cfg_j["unfinished"]
            json.dump(model_cfg_j, f, indent=4)
        cursor += assrec["gpus_shard"]
    log("more_models_than_gpus %d" % more_models_than_gpus)
    cfgs_on_disk = [cfg for cfg in os.listdir(env.DIR_WATCHDOG_D) if cfg.endswith(".cfg") and cfg.startswith("model-")]
    for cfg_fn in cfgs_on_disk:
        if cfg_fn not in allowed_to_exist:
            try:
                os.unlink(os.path.join(env.DIR_WATCHDOG_D, cfg_fn))
            except FileNotFoundError:
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
        except FileNotFoundError:
            pass

    with open(env.CONFIG_INFERENCE, "w") as f:
        json.dump({
            "more_models_than_gpus": more_models_than_gpus,
            **inference_config,
        }, f, indent=4)


def first_run():
    default_config = {
        "model_assign": {
            "CONTRASTcode/3b/multi":  {
                'gpus_shard': 1,
            }
        },
        "completion": "CONTRASTcode/3b/multi",
        "openai_api_enable": False,
    }
    models_to_watchdog_configs(default_config)
