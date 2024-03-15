from typing import List, Tuple

import torch

from self_hosting_machinery.finetune.configuration import supported_models


def get_base_model(model: torch.nn.Module) -> torch.nn.Module:
    if type(model).__name__ == "DeepSpeedEngine":
        model = model.base_model
    if type(model).__name__ in ("LoraModel", "PeftModelForCausalLM"):
        model = model.model
    return model


def map_model_specific_params(
        model_name: str,
        freeze_exceptions: List[str],
        lora_target_modules: List[str]
) -> Tuple[List[str], List[str]]:
    assert model_name in supported_models.config
    model_config = supported_models.config[model_name]
    freeze_exceptions = [mapped for e in freeze_exceptions
                         for mapped in model_config["freeze_exceptions_mapping"][e]]
    lora_target_modules_mapping = [m for modules in lora_target_modules
                                   for m in model_config["lora_target_modules_mapping"][modules]]
    return list(set(freeze_exceptions)), list(set(lora_target_modules_mapping))
