import copy
import hashlib
import os
import json
import time
from pathlib import Path
from typing import List

from refact_utils.scripts import env
from refact_utils.finetune.filtering_defaults import finetune_filtering_defaults
from refact_utils.finetune.train_defaults import finetune_train_defaults

from typing import Any, Dict, Optional, Callable, Union

legacy_finetune_model = "CONTRASTcode/3b/multi"
default_finetune_model = "Refact/1.6B"


def get_run_model_name(run_dir: str) -> str:
    config_json_fn = os.path.join(run_dir, "config.json")
    if not os.path.isfile(config_json_fn):
        raise RuntimeError("get run model name: no config.json found")
    with open(config_json_fn) as f:
        return json.load(f).get("model_name", legacy_finetune_model)


def is_lora_deprecated(checkpoints_dir) -> bool:
    if (checkpoints_dir := Path(checkpoints_dir)).is_dir():
        for d in checkpoints_dir.iterdir():
            load_cp_names = [p.name for p in d.iterdir() if p.suffix in {".pt", ".pth", ".safetensors"}]
            if "adapter_model.safetensors" not in load_cp_names:
                return True
    return False


def get_finetune_runs() -> List[Dict]:
    if not os.path.isdir(env.DIR_LORAS):
        return []

    def model_name(dir_path: str) -> str:
        try:
            if not os.path.isdir(dir_path):
                return ""
            return get_run_model_name(dir_path)
        except RuntimeError:
            return ""

    def get_run_info(dir_path: str, dirname: str) -> Dict:
        checkpoints_dir = os.path.join(dir_path, "checkpoints")
        checkpoints = [
            {"checkpoint_name": checkpoint_dir}
            for checkpoint_dir in sorted(os.listdir(checkpoints_dir))
            if os.path.isdir(os.path.join(checkpoints_dir, checkpoint_dir))
        ] if os.path.isdir(checkpoints_dir) else []

        d = {
            "run_id": dirname,
            "worked_minutes": "0",
            "worked_steps": "0",
            "status": "preparing",
            "model_name": model_name(dir_path),
            "checkpoints": checkpoints,
            "deprecated": is_lora_deprecated(checkpoints_dir),
        }

        if os.path.exists(status_fn := os.path.join(dir_path, "status.json")):
            with open(status_fn, "r") as f:
                d.update(json.load(f))

        if d["status"] not in ["finished", "interrupted", "failed"] and os.path.getmtime(dir_path) + 300 < time.time():
            d["status"] = "interrupted" if (d["status"] in ["preparing"]) else "failed"
        return d

    runs = [
        get_run_info(os.path.join(env.DIR_LORAS, dirname), dirname)
        for dirname in sorted(os.listdir(env.DIR_LORAS))
        if os.path.isdir(os.path.join(env.DIR_LORAS, dirname))
    ]
    return runs


def running_models_and_loras(model_assigner) -> Dict[str, List[str]]:
    data = {
        **model_assigner.models_info,
        **model_assigner.model_assignment,
    }
    result = {}

    def add_result(k: str, model_dict: Dict):
        if model_dict.get('has_completion'):
            result.setdefault('completion', []).append(k)
        if model_dict.get('has_chat'):
            result.setdefault('chat', []).append(k)

    if data.get("openai_api_enable"):
        add_result("gpt-3.5-turbo", {'has_chat': True})
        add_result("gpt-4", {'has_chat': True})

    for k, v in data.get("model_assign", {}).items():
        if model_dict := [d for d in data['models'] if d['name'] == k]:
            model_dict = model_dict[0]

            add_result(k, model_dict)

            if not model_dict.get('has_finetune'):
                continue

            finetune_info = model_dict.get('finetune_info', []) or []
            for run in finetune_info:
                val = f"{k}:{run['run_id']}:{run['checkpoint']}"
                add_result(val, model_dict)

    return result


def get_active_loras(models_db: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    active_loras = {}
    if os.path.exists(env.CONFIG_ACTIVE_LORA):
        active_loras = json.load(open(env.CONFIG_ACTIVE_LORA))
        if "lora_mode" in active_loras:  # NOTE: legacy config format
            active_loras = {
                legacy_finetune_model: active_loras,
            }

    def migrate_active_lora(lora_dict: Dict) -> Dict:
        if lora_dict.get('specific_lora_run_id') and lora_dict.get('specific_checkpoint'):
            lora_dict.update({
                "loras": [{
                    "run_id": lora_dict.get('specific_lora_run_id'),
                    "checkpoint": lora_dict.get('specific_checkpoint'),
                }]
            })
        lora_dict.pop('specific_lora_run_id', None)
        lora_dict.pop('specific_checkpoint', None)
        lora_dict.pop('lora_mode', None)
        lora_dict.pop('model', None)

        return lora_dict

    def get_active_lora(model_name: str, model_info: Dict[str, Any]) -> Dict:
        finetune_model = model_info.get("finetune_model", model_name)
        if finetune_model not in active_loras:
            return {}
        return migrate_active_lora(active_loras[finetune_model])

    return {
        model_name: get_active_lora(model_name, model_info)
        for model_name, model_info in models_db.items()
        if "finetune_model" in model_info or "finetune" in model_info["filter_caps"]
    }


def get_finetune_filter_config(logger: Optional[Callable] = None):
    cfg = {**finetune_filtering_defaults}
    if os.path.exists(env.CONFIG_HOW_TO_FILTER):
        logger("Reading %s" % env.CONFIG_HOW_TO_FILTER)
        cfg.update(**json.load(open(env.CONFIG_HOW_TO_FILTER)))
    return cfg


def get_finetune_config(models_db: Dict[str, Any], logger: Optional[Callable] = None) -> Dict[str, Any]:
    cfg = copy.deepcopy(finetune_train_defaults)
    if os.path.exists(env.CONFIG_FINETUNE):
        if logger is not None:
            logger("Reading %s" % env.CONFIG_FINETUNE)
        cfg.update(**json.load(open(env.CONFIG_FINETUNE)))
    if cfg.get("model_name", None) not in models_db:
        cfg["model_name"] = default_finetune_model
    return cfg


def get_finetune_filter_stat(default: bool = False) -> Dict[str, Any]:
    filter_stats = {
        "filterting_status": "",
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


def get_file_digest(file_path: Union[Path, str]) -> str:
    h = hashlib.sha256()

    with open(file_path, 'rb') as file:
        while True:
            # Reading is buffered, so we can read smaller chunks.
            chunk = file.read(h.block_size)
            if not chunk:
                break
            h.update(chunk)

    return h.hexdigest()
