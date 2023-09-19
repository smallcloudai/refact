import json
import os
import copy

from dataclasses import dataclass, field

from self_hosting_machinery import env
from self_hosting_machinery.webgui.selfhost_webutils import log
from known_models_db.refact_known_models import models_mini_db
from known_models_db.refact_toolbox_db import modelcap_records

from typing import List, Dict, Set, Any


__all__ = ["ModelAssigner"]


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
        inference_config = self._integrations_inference_setup(inference_config)

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

    def _model_inference_setup(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
        gpus = self.gpus["gpus"]
        model_groups = self._model_assign_to_groups(inference_config["model_assign"])
        # This must work or installation is bad
        model_cfg_template = json.load(open(os.path.join(env.DIR_WATCHDOG_TEMPLATES, "model.cfg")))
        cursor = 0
        allowed_to_exist = []
        required_memory_exceed_available = False
        more_models_than_gpus = False
        for model_group in model_groups:
            models_message = ' '.join([f"'{model_name}'" for model_name in model_group.model_assign.keys()])
            log(f"assign models {models_message}, cursor {cursor}, gpus_shard {model_group.gpus_shard()}")
            if cursor + model_group.gpus_shard() > len(gpus):
                more_models_than_gpus = True
                break
            for model_name, assignment in model_group.model_assign.items():
                for idx, model_cursor in enumerate(range(cursor, cursor + assignment["gpus_shard"])):
                    cfg_out = f"model-{model_name.lower().replace('/', '-')}-{idx}.cfg"
                    allowed_to_exist.append(cfg_out)
                    with open(os.path.join(env.DIR_WATCHDOG_D, cfg_out), "w") as f:
                        model_cfg_j = copy.deepcopy(model_cfg_template)
                        model_cfg_j["command_line"].append("--model")
                        model_cfg_j["command_line"].append(model_name)
                        model_cfg_j["gpus"] = list(range(model_cursor, model_cursor + assignment["gpus_shard"]))
                        model_cfg_j["share_gpu"] = assignment.get("share_gpu", False)
                        del model_cfg_j["unfinished"]
                        json.dump(model_cfg_j, f, indent=4)
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

    def _integrations_inference_setup(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
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

        return inference_config

    def first_run(self):
        default_config = {
            "model_assign": {
                "Refact/1.6B": {
                    'gpus_shard': 1,
                    'share_gpu': False,
                }
            },
            "completion": "Refact/1.6B",
            "openai_api_enable": False,
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

        chat_caps = _capabilities("chat")
        toolbox_caps = _capabilities("toolbox")
        for k, rec in self.models_db.items():
            if rec.get("hidden", False):
                continue
            info.append({
                "name": k,
                "has_completion": bool("completion" in rec["filter_caps"]),
                "has_finetune": bool("finetune" in rec["filter_caps"]),
                "has_toolbox": bool(toolbox_caps.intersection(rec["filter_caps"])),
                "has_chat": bool(rec["chat_scratchpad_class"]) and bool(chat_caps.intersection(rec["filter_caps"])),
            })
        return {"models": info}

    @property
    def model_assignment(self):
        if os.path.exists(env.CONFIG_INFERENCE):
            j = json.load(open(env.CONFIG_INFERENCE, "r"))
        else:
            j = {"model_assign": {}}
        j["model_assign"] = {
            model: v for model, v in j["model_assign"].items()
            if model in self.models_db
        }
        return j
