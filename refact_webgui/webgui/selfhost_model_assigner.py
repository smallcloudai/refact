import json
import os
import copy

from dataclasses import dataclass, field

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_active_loras
from refact_webgui.webgui.selfhost_webutils import log
from known_models_db.refact_known_models import models_mini_db, passthrough_mini_db
from known_models_db.refact_toolbox_db import modelcap_records

from typing import List, Dict, Set, Any


__all__ = ["ModelAssigner"]


ALLOWED_N_CTX = [2 ** p for p in range(10, 20)]


def has_context_switch(filter_caps: List[str]) -> bool:
    return "chat" in filter_caps or "completion" in filter_caps


def get_default_n_ctx(model_name: str, model_info: Dict[str, Any]) -> int:
    if "T" in model_info:
        return model_info["T"]
    if "n_ctx" in model_info:
        return model_info["n_ctx"]
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


class ModelAssigner:

    @property
    def models_db(self) -> Dict[str, Any]:
        return models_mini_db

    @property
    def passthrough_mini_db(self) -> Dict[str, Any]:
        return passthrough_mini_db

    @property
    def models_db_with_passthrough(self) -> Dict[str, Any]:
        return {**self.models_db, **self.passthrough_mini_db}

    @property
    def models_caps_db(self) -> List:
        return modelcap_records.db

    def _model_assign_to_groups(self, model_assign: Dict[str, Dict]) -> List[ModelGroup]:
        model_groups: List[ModelGroup] = []
        shared_group = ModelGroup()
        for model_name, assignment in model_assign.items():
            if model_name not in self.models_db.keys():
                log(f"unknown model '{model_name}', skipping")
                continue
            if assignment["gpus_shard"] not in [1, 2, 4]:
                log(f"invalid shard count {assignment['gpus_shard']}, skipping '{model_name}'")
                continue
            if self.models_db[model_name]["backend"] not in ["transformers"] and assignment["gpus_shard"] > 1:
                log(f"sharding not supported for '{self.models_db['backend']}' backend, skipping '{model_name}'")
                continue
            if assignment.get("share_gpu", False):
                if not shared_group.model_assign:
                    model_groups.append(shared_group)
                shared_group.model_assign[model_name] = assignment
            else:
                model_groups.append(ModelGroup({model_name: assignment}))
        return model_groups

    def models_to_watchdog_configs(self, inference_config=None):
        if inference_config is None:
            inference_config = self.model_assignment

        inference_config = self._model_assign_filter(inference_config)
        inference_config = self._model_inference_setup(inference_config)

        with open(env.CONFIG_INFERENCE + ".tmp", "w") as f:
            json.dump(inference_config, f, indent=4)
        os.rename(env.CONFIG_INFERENCE + ".tmp", env.CONFIG_INFERENCE)

    def _model_assign_filter(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
        inference_config["model_assign"] = {
            model_name: model_cfg
            for model_name, model_cfg in inference_config["model_assign"].items()
            if model_name in self.models_db and not self.models_db[model_name].get("hidden")
        }
        return inference_config

    def _model_cfg_template(self) -> Dict:
        return json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))

    def _model_inference_setup(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
        gpus = self.gpus["gpus"]
        model_groups = self._model_assign_to_groups(inference_config["model_assign"])
        cursor = 0
        allowed_to_exist = []
        required_memory_exceed_available = False
        more_models_than_gpus = False
        for model_group in model_groups:
            models_message = ' '.join([f"'{model_name}'" for model_name in model_group.model_assign.keys()])
            log(f"assign models {models_message}, cursor {cursor}, gpus_shard {model_group.gpus_shard()}")
            next_cursor = cursor + model_group.gpus_shard()
            if cursor + model_group.gpus_shard() > len(gpus):
                more_models_than_gpus = True
                break
            for model_name, assignment in model_group.model_assign.items():
                for idx, model_cursor in enumerate(range(cursor, next_cursor, assignment["gpus_shard"])):
                    cfg_out = f"model-{model_name.lower().replace('/', '-')}-{idx}.cfg"
                    allowed_to_exist.append(cfg_out)
                    fn = os.path.join(env.DIR_WATCHDOG_D, cfg_out)
                    with open(fn + ".tmp", "w") as f:
                        model_cfg_j = self._model_cfg_template()
                        model_cfg_j["command_line"].append("--model")
                        model_cfg_j["command_line"].append(model_name)
                        model_cfg_j["gpus"] = list(range(model_cursor, model_cursor + assignment["gpus_shard"]))
                        model_cfg_j["share_gpu"] = assignment.get("share_gpu", False)
                        del model_cfg_j["unfinished"]
                        json.dump(model_cfg_j, f, indent=4)
                    os.rename(fn + ".tmp", fn)
            for _ in range(model_group.gpus_shard()):
                if gpus[cursor]["mem_total_mb"] < model_group.required_memory_mb(self.models_db):
                    required_memory_exceed_available = True
                cursor += 1
        log("required_memory_exceed_available %d" % required_memory_exceed_available)
        log("more_models_than_gpus %d" % more_models_than_gpus)
        cfgs_on_disk = [cfg for cfg in os.listdir(env.DIR_WATCHDOG_D) if
                        cfg.endswith(".cfg") and cfg.startswith("model-")]
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
                "Refact/1.6B": {
                    'gpus_shard': 1,
                    'share_gpu': True,
                },
                "thenlper/gte-base": {
                    'gpus_shard': 1,
                    'share_gpu': True,
                },
            },
            "openai_api_enable": False,
            "anthropic_api_enable": False,
        }
        self.models_to_watchdog_configs(default_config)

    @property
    def gpus(self):
        if os.path.exists(env.CONFIG_ENUM_GPUS):
            result = json.load(open(env.CONFIG_ENUM_GPUS, "r"))
        else:
            result = {"gpus": []}
        if os.path.exists(env.CONFIG_BUSY_GPUS):
            statuses = json.load(open(env.CONFIG_BUSY_GPUS, "r"))
            if isinstance(statuses["gpus"], list):  # convert old format to new
                statuses["gpus"] = {
                    idx: [status]
                    for idx, status in enumerate(statuses["gpus"])
                    if status
                }
            statuses["gpus"] = {
                int(k): v for k, v in statuses["gpus"].items()
            }
            for idx, gpu_info in enumerate(result["gpus"]):
                gpu_info["statuses"] = statuses["gpus"].get(idx, [])
        return result

    @property
    def models_info(self):
        info = []

        def _capabilities(func_type: str) -> Set:
            return {
                capability
                for func in self.models_caps_db
                for capability in func.model
                if func.type == func_type
            }

        toolbox_caps = _capabilities("toolbox")
        active_loras = get_active_loras(self.models_db)
        for k, rec in self.models_db.items():
            if rec.get("hidden", False):
                continue
            finetune_info = None
            if k in active_loras:
                finetune_info = [
                    {
                        "run_id": l["run_id"],
                        "checkpoint": l["checkpoint"],
                    } for l in active_loras[k].get('loras', [])
                ]
            has_finetune = bool("finetune" in rec["filter_caps"])
            finetune_model = rec.get("finetune_model", k if has_finetune else None)
            default_n_ctx = get_default_n_ctx(k, rec)
            available_n_ctx = []
            if has_context_switch(rec["filter_caps"]):
                available_n_ctx = list(filter(lambda n_ctx: n_ctx <= default_n_ctx, ALLOWED_N_CTX))
                assert default_n_ctx in available_n_ctx, \
                    f"default n_ctx {default_n_ctx} not in {available_n_ctx}"
            info.append({
                "name": k,
                "backend": rec["backend"],
                "finetune_info": finetune_info,
                "finetune_model": finetune_model,
                "has_completion": bool("completion" in rec["filter_caps"]),
                "has_finetune": has_finetune,
                "has_toolbox": bool(toolbox_caps.intersection(rec["filter_caps"])),
                "has_embeddings": bool("embeddings" in rec["filter_caps"]),
                "has_chat": bool("chat" in rec["filter_caps"]),
                "has_sharding": rec["backend"] in ["transformers"],
                "default_n_ctx": default_n_ctx,
                "available_n_ctx": available_n_ctx,
                "is_deprecated": bool(rec.get("deprecated", False)),
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

        j["model_assign"] = {
            model: _set_n_ctx(model, v) for model, v in j["model_assign"].items()
            if model in self.models_db
        }
        return j

    def config_inference_mtime(self) -> int:
        if os.path.exists(env.CONFIG_INFERENCE):
            try:
                return int(os.path.getmtime(env.CONFIG_INFERENCE))
            except OSError:
                return 0
        return 0
