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


def is_checkpoint_deprecated(checkpoint_dir: Path) -> bool:
    load_cp_names = [p.name for p in checkpoint_dir.iterdir() if p.suffix in {".pt", ".pth", ".safetensors"}]
    return "adapter_model.safetensors" not in load_cp_names


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

        deprecated = any([
            is_checkpoint_deprecated(Path(checkpoints_dir) / checkpoint_info["checkpoint_name"])
            for checkpoint_info in checkpoints
        ])
        d = {
            "run_id": dirname,
            "worked_minutes": "0",
            "worked_steps": "0",
            "status": "preparing",
            "model_name": model_name(dir_path),
            "checkpoints": checkpoints,
            "deprecated": deprecated,
        }
        # TODO: integrate
        # ftune_cfg_j["save_status"] = os.path.join(env.DIR_LORAS, run_id, "watchdog_status.out")

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
    result = {
        "completion": [],
        "chat": [],
    }

    def add_result(k: str, model_dict: Dict):
        if model_dict.get('has_completion'):
            result['completion'].append(k)
        if model_dict.get('has_chat'):
            result['chat'].append(k)

    if data.get("openai_api_enable"):
        add_result("gpt-3.5-turbo", {'has_chat': True})
        add_result("gpt-4", {'has_chat': True})

    if data.get('anthropic_api_enable'):
        add_result('claude-instant-1.2', {'has_chat': True})
        add_result('claude-2.1', {'has_chat': True})

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
        if model_name not in active_loras:
            return {}
        active_lora = migrate_active_lora(active_loras[model_name]).get("loras", [])
        filtered_active_lora = []
        for lora_info in active_lora:
            checkpoint_dir = Path(env.DIR_LORAS) / lora_info["run_id"] / "checkpoints" / lora_info["checkpoint"]
            if not checkpoint_dir.exists():
                continue
            if model_info.get("finetune_model", model_name) != model_name and is_checkpoint_deprecated(checkpoint_dir):
                continue
            filtered_active_lora.append(lora_info)
        return {
            "loras": filtered_active_lora
        }

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


def get_finetune_config(models_db: Dict) -> Dict[str, Any]:
    cfg = copy.deepcopy(finetune_train_defaults)
    if os.path.exists(env.CONFIG_FINETUNE):
        cfg.update(**json.load(open(env.CONFIG_FINETUNE)))
    if cfg.get("model_name", None) not in models_db:
        cfg["model_name"] = default_finetune_model
    return cfg


def get_finetune_filter_stat(pname: str, default: bool = False) -> Dict[str, Any]:
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
    if not default and os.path.isfile(env.PP_CONFIG_FINETUNE_FILTER_STAT(pname)):
        filter_stats.update(**json.load(open(env.PP_CONFIG_FINETUNE_FILTER_STAT(pname))))
    return filter_stats


def _get_status_by_watchdog(pname: str) -> (str, str):
    # this returns:
    # "linguist", "starting"
    # "filter", "interrupted"
    if os.path.isfile(env.PP_SCAN_STATUS(pname)):
        mtime = os.path.getmtime(env.PP_SCAN_STATUS(pname))
        if mtime + 600 > time.time():
            d = json.load(open(env.PP_SCAN_STATUS(pname), "r"))
            return d["prog"], d["status"]
    return "", "idle"


def get_prog_and_status_for_ui(pname: str) -> (str, str):
    # def get_sources_stats():
    #     scan_stats = {
    #         "scan_status": "idle",
    #     }
    #     if os.path.isfile(env.CONFIG_PROCESSING_STATS):
    #         scan_stats.update(**json.load(open(env.CONFIG_PROCESSING_STATS, "r")))
    #     return scan_stats

    prog, status = _get_status_by_watchdog(pname)

    # if os.path.exists(env.FLAG_LAUNCH_PROCESS_UPLOADS):
    #     return "prog_linguist", "starting"

    # if os.path.exists(env.FLAG_LAUNCH_FINETUNE_FILTER_ONLY):
    #     return "prog_filter", "starting"

    # if os.path.exists(env.FLAG_LAUNCH_FINETUNE):
    #     return "prog_ftune", "starting"

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
