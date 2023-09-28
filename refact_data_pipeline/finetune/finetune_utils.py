import os
import json
import time

from refact_data_pipeline.finetune.finetune_train_defaults import finetune_train_defaults

from self_hosting_machinery import env

from typing import Any, Dict, Optional, Callable


legacy_finetune_model = "CONTRASTcode/3b/multi"
default_finetune_model = "Refact/1.6B"


def get_run_model_name(run_dir: str) -> str:
    config_json_fn = os.path.join(run_dir, "config.json")
    if not os.path.isfile(config_json_fn):
        raise RuntimeError("get run model name: no config.json found")
    with open(config_json_fn) as f:
        return json.load(f).get("model_name", legacy_finetune_model)


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
            d["model_name"] = get_run_model_name(dir_path)
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


def get_active_loras(models_db: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    active_loras = {}
    if os.path.exists(env.CONFIG_ACTIVE_LORA):
        active_loras = json.load(open(env.CONFIG_ACTIVE_LORA))
        if "lora_mode" in active_loras:  # NOTE: legacy config format
            active_loras = {
                legacy_finetune_model: active_loras,
            }
    return {
        model_name: {
            "lora_mode": "latest-best",
            **active_loras.get(model_name, {}),
        }
        for model_name, model_info in models_db.items()
        if "finetune" in model_info["filter_caps"]
    }


def get_finetune_config(logger: Optional[Callable] = None) -> Dict[str, Any]:
    cfg = {
        "model_name": default_finetune_model,
        **finetune_train_defaults
    }
    if os.path.exists(env.CONFIG_FINETUNE):
        if logger is not None:
            logger("Reading %s" % env.CONFIG_FINETUNE)
        cfg.update(**json.load(open(env.CONFIG_FINETUNE)))
    return cfg


def get_finetune_filter_stat(default: bool = False) -> Dict[str, Any]:
    filter_stats = {
        "filterting_status": "",
        "error": "",
        "total_steps": 0,
        "worked_steps": 0,
        "worked_minutes": 0,
        "eta_minutes": 0,
        "accepted": 0,
        "rejected": 0,
        "avg_loss": 0.0,
    }
    if not default and os.path.isfile(env.CONFIG_FINETUNE_FILTER_STAT):
        filter_stats.update(**json.load(open(env.CONFIG_FINETUNE_FILTER_STAT)))
    return filter_stats


def _get_status_by_watchdog() -> (str, str):
    # this returns:
    # "linguist", "starting"
    # "filter", "interrupted"
    # "ftune", "working"
    if os.path.isfile(env.CONFIG_FINETUNE_STATUS):
        mtime = os.path.getmtime(env.CONFIG_FINETUNE_STATUS)
        if mtime + 600 > time.time():
            d = json.load(open(env.CONFIG_FINETUNE_STATUS))
            return d["prog"], d["status"]
    return "", "idle"


def get_prog_and_status_for_ui() -> (str, str):
    # def get_sources_stats():
    #     scan_stats = {
    #         "scan_status": "idle",
    #     }
    #     if os.path.isfile(env.CONFIG_PROCESSING_STATS):
    #         scan_stats.update(**json.load(open(env.CONFIG_PROCESSING_STATS, "r")))
    #     return scan_stats

    prog, status = _get_status_by_watchdog()

    if os.path.exists(env.FLAG_LAUNCH_PROCESS_UPLOADS):
        return "prog_linguist", "starting"

    if os.path.exists(env.FLAG_LAUNCH_FINETUNE_FILTER_ONLY):
        return "prog_filter", "starting"

    if os.path.exists(env.FLAG_LAUNCH_FINETUNE):
        return "prog_ftune", "starting"

    return prog, status
