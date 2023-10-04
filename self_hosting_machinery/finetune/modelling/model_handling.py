import importlib
from collections import deque
from functools import partial
from pathlib import Path
from typing import List, Tuple, Optional

import torch as th
import torch.nn.functional as F
from transformers import AutoModelForCausalLM, AutoTokenizer

from refact_encoding import RefactEncoding
from refact_models.checkpoint_loader import load_config
from refact_models.lora import LoraMixin
from self_hosting_machinery.finetune.configuration import supported_models




def model_forward(
        model: th.nn.Module,
        input: th.Tensor,
        low_gpu_mem_mode: bool,
) -> th.Tensor:
    if low_gpu_mem_mode:
        model.gradient_checkpointing_enable()

        def make_inputs_require_grad(module, input, output):
            output.requires_grad_(True)

        model.get_input_embeddings().register_forward_hook(make_inputs_require_grad)
    else:
        model.gradient_checkpointing_disable()
    logits = model.forward(
        input,
        return_dict=False, output_attentions=False, output_hidden_states=False
    )[0]
    return logits


def map_model_specific_params(
        model_name: str,
        freeze_exceptions: List[str],
        lora_target_modules: List[str]
) -> Tuple[List[str], List[str]]:
    assert model_name in supported_models.config
    model_config = supported_models.config[model_name]
    freeze_exceptions = [model_config["freeze_exceptions_mapping"][e] for e in freeze_exceptions]
    lora_target_modules_mapping = [m for modules in lora_target_modules
                                   for m in model_config["lora_target_modules_mapping"][modules]]
    return list(set(freeze_exceptions)), list(set(lora_target_modules_mapping))
