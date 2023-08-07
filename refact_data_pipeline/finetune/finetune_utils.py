import os
import json

from known_models_db.refact_known_models import models_mini_db
from refact_data_pipeline.finetune.finetune_train_defaults import finetune_train_defaults
from self_hosting_machinery import env

from typing import Any, Dict, Optional, Callable


default_finetune_model = "CONTRASTcode/3b/multi"


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
