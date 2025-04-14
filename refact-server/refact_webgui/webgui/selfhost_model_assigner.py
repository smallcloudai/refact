import json
import os

from dataclasses import dataclass, field

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_active_loras
from refact_utils.huggingface.utils import is_hf_hub_offline
from refact_utils.huggingface.utils import get_repo_status
from refact_webgui.webgui.selfhost_webutils import log
from refact_known_models import models_mini_db

from pathlib import Path
from typing import List, Dict, Any, Set, Optional


__all__ = ["ModelAssigner"]


ALLOWED_N_CTX = [2 ** p for p in range(10, 20)]
ALLOWED_GPUS_SHARD = [2 ** p for p in range(10)]


def has_context_switch(filter_caps: List[str]) -> bool:
    return "chat" in filter_caps or "completion" in filter_caps


def get_default_n_ctx(model_name: str, model_info: Dict[str, Any]) -> int:
    if "T" in model_info:
        return model_info["T"]
    raise ValueError(f"context size is not specified for '{model_name}'")


@dataclass
class ModelGroup:
    model_assign: Dict[str, Dict] = field(default_factory=dict)

    def required_memory_mb(self, models_db: Dict[str, Any]) -> int:
        return sum(
            models_db[model_name].get("required_memory_mb", 0)
            for model_name in self.model_assign.keys()
        )

    def gpus_shard(self) -> int:
        if not self.model_assign:
            return 0
        return max([rec["gpus_shard"] for rec in self.model_assign.values()])


@dataclass
class ModelWatchdogDConfig:
    backend: str
    model_name: str
    gpus: List[int]
    share_gpu: bool
    n_ctx: Optional[int] = None
    has_loras: bool = False

    def dump(self, model_cfg_j: Dict) -> str:
        model_cfg_j["command_line"].extend(["--model", self.model_name])
        if self.backend not in ["transformers"]:
            if self.n_ctx is not None:
                model_cfg_j["command_line"].extend(["--n-ctx", self.n_ctx])
            if not self.has_loras:
                model_cfg_j["command_line"].append("--loraless")

        model_cfg_j["gpus"] = self.gpus
        model_cfg_j["share_gpu"] = self.share_gpu
        model_cfg_j["inform_about_device_status"] = True
        del model_cfg_j["unfinished"]

        if self.gpus:
            devices_name_list = [f"{gpu:02d}" for gpu in self.gpus]
        else:
            devices_name_list = ["cpu"]
        cfg_fn = "-".join([
            "model",
            self.model_name.lower().replace('/', '-'),
            *devices_name_list
        ]) + ".cfg"

        fn = os.path.join(env.DIR_WATCHDOG_D, cfg_fn)
        with open(fn + ".tmp", "w") as f:
            json.dump(model_cfg_j, f, indent=4)
        os.rename(fn + ".tmp", fn)

        return cfg_fn


class ModelAssigner:

    def __init__(self):
        self._models_repo_status = {
            model_name: get_repo_status(model_info["model_path"]).value
            for model_name, model_info in self.models_db.items()
        }

    @property
    def shard_gpu_backends(self) -> Set[str]:
        return {"transformers"}

    @property
    def share_gpu_backends(self) -> Set[str]:
        return {"transformers"}

    @property
    def models_db(self) -> Dict[str, Any]:
        return models_mini_db

    def to_completion_model_record(self, model_name: str, model_info: Dict[str, Any]) -> Dict[str, Any]:
        return {
            "n_ctx": min(self.model_assignment["model_assign"].get(model_name, {}).get("n_ctx", model_info["T"]), model_info["T"]),
            "supports_scratchpads": model_info["supports_scratchpads"]["completion"],
        }

    def to_chat_model_record(self, model_name: str, model_info: Dict[str, Any]) -> Dict[str, Any]:
        return {
            "n_ctx": min(self.model_assignment["model_assign"].get(model_name, {}).get("n_ctx", model_info["T"]), model_info["T"]),
            "supports_scratchpads": model_info["supports_scratchpads"]["chat"],
        }

    def _model_assign_to_groups(self, model_assign: Dict[str, Dict]) -> List[ModelGroup]:
        model_groups: List[ModelGroup] = []
        shared_group = ModelGroup()
        cpu_group = ModelGroup()
        for model_name, assignment in model_assign.items():
            if model_name not in self.models_db.keys():
                log(f"unknown model '{model_name}', skipping")
                continue
            model_dict = self.models_db[model_name]
            if (assignment["gpus_shard"] > 0
                    and assignment["gpus_shard"] not in ALLOWED_GPUS_SHARD
                    or assignment["gpus_shard"] > model_dict.get("max_gpus_shard", assignment["gpus_shard"])):
                log(f"invalid shard count {assignment['gpus_shard']}, skipping '{model_name}'")
                continue
            if (assignment["gpus_shard"] > 1 and
                    model_dict["backend"] not in self.shard_gpu_backends):
                log(f"sharding not supported for '{model_dict['backend']}' backend, skipping '{model_name}'")
                continue
            if model_dict.get("cpu"):
                cpu_group.model_assign[model_name] = assignment
            elif (assignment.get("share_gpu", False)
                    and model_dict["backend"] in self.share_gpu_backends):
                if not shared_group.model_assign:
                    model_groups.append(shared_group)
                shared_group.model_assign[model_name] = assignment
            elif model_dict.get("cpu"):
                cpu_group.model_assign[model_name] = assignment
            else:
                model_groups.append(ModelGroup({model_name: assignment}))
        if cpu_group.model_assign:
            model_groups = [cpu_group, *model_groups]
        return model_groups

    def models_to_watchdog_configs(self, inference_config=None):
        if inference_config is None:
            inference_config = self.model_assignment

        inference_config["model_assign"] = self._model_assign_filter(inference_config["model_assign"])
        inference_config["model_assign"] = self._share_gpu_filter(inference_config["model_assign"])
        inference_config = self._model_inference_setup(inference_config)

        with open(env.CONFIG_INFERENCE + ".tmp", "w") as f:
            json.dump(inference_config, f, indent=4)
        os.rename(env.CONFIG_INFERENCE + ".tmp", env.CONFIG_INFERENCE)

    def _model_assign_filter(self, model_assign: Dict[str, Any]) -> Dict[str, Any]:
        return {
            model_name: model_cfg
            for model_name, model_cfg in model_assign.items()
            if model_name in self.models_db and not self.models_db[model_name].get("hidden")
        }

    def _share_gpu_filter(self, model_assign: Dict[str, Any]) -> Dict[str, Any]:
        def _update_share_gpu(model: str, record: Dict) -> Dict:
            allow_share_gpu = self.models_db[model]["backend"] in self.share_gpu_backends
            record["share_gpu"] = record.get("share_gpu", False) and allow_share_gpu
            return record

        return {
            model_name: _update_share_gpu(model_name, model_cfg)
            for model_name, model_cfg in model_assign.items()
        }

    @staticmethod
    def has_available_weights(model_path: str) -> bool:
        weights_dir = Path(env.DIR_WEIGHTS) / f"models--{model_path.replace('/', '--')}"
        return weights_dir.exists()

    @property
    def _model_cfg_template(self) -> Dict:
        return json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))

    def _has_loras(self, model_name: str) -> bool:
        active_loras = get_active_loras(self.models_db)
        return bool(active_loras.get(model_name, {}).get("loras", []))

    def _model_inference_setup(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
        gpus = self.devices["gpus"]
        model_groups = self._model_assign_to_groups(inference_config["model_assign"])
        cursor = 0
        required_memory_exceed_available = False
        more_models_than_gpus = sum([mg.gpus_shard() for mg in model_groups]) > len(gpus)

        model_configs = []
        for model_group in model_groups:
            next_cursor = cursor + model_group.gpus_shard()
            if next_cursor > len(gpus):
                break

            for model_name, assignment in model_group.model_assign.items():
                if assignment["gpus_shard"] == 0:
                    # NOTE: CPU case
                    model_configs.append(ModelWatchdogDConfig(
                        backend=self.models_db.get(model_name, {}).get("backend", ""),
                        model_name=model_name,
                        gpus=[],
                        share_gpu=assignment.get("share_gpu", False),
                        n_ctx=assignment.get("n_ctx", None),
                        has_loras=self._has_loras(model_name),
                    ))
                    continue
                for model_cursor in range(cursor, next_cursor, assignment["gpus_shard"]):
                    model_configs.append(ModelWatchdogDConfig(
                        backend=self.models_db.get(model_name, {}).get("backend", ""),
                        model_name=model_name,
                        gpus=list(range(model_cursor, model_cursor + assignment["gpus_shard"])),
                        share_gpu=assignment.get("share_gpu", False),
                        n_ctx=assignment.get("n_ctx", None),
                        has_loras=self._has_loras(model_name),
                    ))
            for _ in range(model_group.gpus_shard()):
                if gpus[cursor]["mem_total_mb"] < model_group.required_memory_mb(self.models_db):
                    required_memory_exceed_available = True
                cursor += 1

        # dump configs
        allowed_to_exist = set()
        for config in model_configs:
            fn = config.dump(self._model_cfg_template)
            allowed_to_exist.add(fn)
            log(f"assign model {config.model_name}, gpus {config.gpus}: {fn}")

        log("required_memory_exceed_available %d" % required_memory_exceed_available)
        log("more_models_than_gpus %d" % more_models_than_gpus)
        cfgs_on_disk = [cfg for cfg in os.listdir(env.DIR_WATCHDOG_D) if
                        cfg.endswith(".cfg") and cfg.startswith("model-")]

        # remove configs that are not allowed now
        for cfg_fn in cfgs_on_disk:
            if cfg_fn not in allowed_to_exist:
                try:
                    os.unlink(os.path.join(env.DIR_WATCHDOG_D, cfg_fn))
                except FileNotFoundError:
                    pass

        return {
            **inference_config,
            "required_memory_exceed_available": required_memory_exceed_available,
            "more_models_than_gpus": more_models_than_gpus,
        }

    def first_run(self):
        default_config = {
            "model_assign": {
                "qwen2.5/coder/1.5b/base": {
                    'gpus_shard': 1,
                    'share_gpu': True,
                },
                "thenlper/gte-base/cpu": {
                    'gpus_shard': 0,
                    'share_gpu': False,
                },
            },
        }
        self.models_to_watchdog_configs(default_config)

    @property
    def devices(self):
        result = json.load(open(env.CONFIG_ENUM_DEVICES, "r"))
        if os.path.exists(env.CONFIG_BUSY_DEVICES):
            statuses = json.load(open(env.CONFIG_BUSY_DEVICES, "r"))
            result["cpu"]["statuses"] = statuses["cpu"]
            statuses["gpus"] = {
                int(k): v for k, v in statuses["gpus"].items()
            }
            for idx, gpu_info in enumerate(result["gpus"]):
                gpu_info["statuses"] = statuses["gpus"].get(idx, [])
        else:
            result["cpu"]["statuses"] = []
            for idx, gpu_info in enumerate(result["gpus"]):
                gpu_info["statuses"] = []
        return result

    @property
    def models_info(self):
        info = []

        is_hf_offline = is_hf_hub_offline()
        active_loras = get_active_loras(self.models_db)
        for k, rec in self.models_db.items():
            if rec.get("hidden", False):
                continue
            finetune_info = [
                {
                    "run_id": l["run_id"],
                    "checkpoint": l["checkpoint"],
                } for l in active_loras.get(k, {}).get('loras', [])
            ]
            has_finetune = bool("finetune" in rec["filter_caps"])
            finetune_model = rec.get("finetune_model", k if has_finetune else None)
            default_n_ctx = get_default_n_ctx(k, rec)
            available_n_ctx = []
            if has_context_switch(rec["filter_caps"]):
                available_n_ctx = list(filter(lambda n_ctx: n_ctx <= default_n_ctx, ALLOWED_N_CTX))
                assert default_n_ctx in available_n_ctx, \
                    f"default n_ctx {default_n_ctx} not in {available_n_ctx}"
            has_share_gpu = rec["backend"] in self.share_gpu_backends
            available_shards = [1]
            if rec.get("cpu"):
                has_share_gpu = False
                available_shards = [0]
            elif rec["backend"] in self.shard_gpu_backends:
                max_gpus = len(self.devices["gpus"])
                max_available_shards = min(max_gpus, rec.get("max_gpus_shard", max_gpus))
                available_shards = [
                    gpus_shard for gpus_shard in ALLOWED_GPUS_SHARD
                    if gpus_shard <= max_available_shards
                ]
            info.append({
                "name": k,
                "backend": rec["backend"],
                "finetune_info": finetune_info,
                "finetune_model": finetune_model,
                "has_completion": bool("completion" in rec["filter_caps"]),
                "has_finetune": has_finetune,
                "has_embeddings": bool("embeddings" in rec["filter_caps"]),
                "has_chat": bool("chat" in rec["filter_caps"]),
                "has_share_gpu": has_share_gpu,
                "default_n_ctx": default_n_ctx,
                "available_n_ctx": available_n_ctx,
                "available_shards": available_shards,
                "is_deprecated": bool(rec.get("deprecated", False)),
                "repo_status": self._models_repo_status[k],
                "repo_url": f"https://huggingface.co/{rec['model_path']}",
                "is_hf_offline": is_hf_offline,
                "has_weights_loaded": self.has_available_weights(rec['model_path']),
                "model_path": rec['model_path'],
            })
        return {"models": info}

    @property
    def model_assignment(self):
        if os.path.exists(env.CONFIG_INFERENCE):
            j = json.load(open(env.CONFIG_INFERENCE, "r"))
        else:
            j = {"model_assign": {}}

        def _set_n_ctx(model: str, record: Dict) -> Dict:
            default_n_ctx = get_default_n_ctx(model, self.models_db[model])
            if not has_context_switch(self.models_db[model].get("filter_caps", [])):
                record["n_ctx"] = default_n_ctx
                return record
            n_ctx = record.get("n_ctx", default_n_ctx)
            if n_ctx not in ALLOWED_N_CTX or n_ctx > default_n_ctx:
                n_ctx = default_n_ctx
            record["n_ctx"] = n_ctx
            return record

        j["model_assign"] = self._share_gpu_filter({
            model: _set_n_ctx(model, v)
            for model, v in j["model_assign"].items()
            if model in self.models_db
        })

        return j

    def config_inference_mtime(self) -> int:
        if os.path.exists(env.CONFIG_INFERENCE):
            try:
                return int(os.path.getmtime(env.CONFIG_INFERENCE))
            except OSError:
                return 0
        return 0
