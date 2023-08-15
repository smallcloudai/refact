import os
import json
import time

from known_models_db.refact_known_models import models_mini_db
from refact_data_pipeline.finetune.finetune_train_defaults import finetune_train_defaults

from self_hosting_machinery.scripts import best_lora
from self_hosting_machinery import env

from typing import Any, Dict, Optional, Callable


default_finetune_model = "CONTRASTcode/3b/multi"


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


def get_active_loras() -> Dict[str, Dict[str, Any]]:
    active_loras = {}
    if os.path.exists(env.CONFIG_ACTIVE_LORA):
        active_loras = json.load(open(env.CONFIG_ACTIVE_LORA))
        if "lora_mode" in active_loras:  # NOTE: old config format
            active_loras = {
                default_finetune_model: active_loras,
            }
    return {
        model_name: {
            "lora_mode": "latest-best",
            **active_loras.get(model_name, {}),
        }
        for model_name, model_info in models_mini_db.items()
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


def get_finetune_filter_stats(default: bool = False) -> Dict[str, Any]:
    filter_stats = {
        "started_ts": 0,
        "total_steps": 0,
        "worked_steps": 0,
        "worked_minutes": 0,
        "eta_minutes": 0,
        "accepted": 0,
        "rejected": 0,
        "avg_loss": 0.0,
        "status": "idle",
    }
    if not default and os.path.isfile(env.CONFIG_FINETUNE_FILTER_STATS):
        filter_stats.update(**json.load(open(env.CONFIG_FINETUNE_FILTER_STATS)))
    return filter_stats


def get_finetune_step() -> Optional[str]:

    def get_sources_stats():
        scan_stats = {
            "scan_status": "idle",
        }
        if os.path.isfile(env.CONFIG_PROCESSING_STATS):
            scan_stats.update(**json.load(open(env.CONFIG_PROCESSING_STATS, "r")))
        return scan_stats

    if get_sources_stats()["scan_status"] in ["working"]:
        return "sources"

    if get_finetune_filter_stats()["status"] in ["starting", "filtering"]:
        return "filter"

    if get_finetune_runs()[1]:
        return "finetune"

    return None
