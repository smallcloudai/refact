import json
import logging
from pathlib import Path

import torch
from torch import nn

from transformers import modeling_utils
from huggingface_hub import hf_hub_download
from transformers.models.gpt_bigcode.modeling_gpt_bigcode import GPTBigCodeConfig
from transformers.models.gpt_bigcode.modeling_gpt_bigcode import GPTBigCodeForCausalLM

from code_contrast.encoding import SMCEncoding
from code_contrast.modeling.quant import QuantLinear

from typing import Optional, Tuple, Any, List


def disable_torch_init():
    def noop(*args, **kwargs):
        pass

    torch.nn.init.kaiming_uniform_ = noop
    torch.nn.init.uniform_ = noop
    torch.nn.init.normal_ = noop
    modeling_utils._init_weights = False


def load_filename(filename: str, repo_id: str, cache_dir: str):
    args = dict(
        repo_id=repo_id,
        filename=filename,
        cache_dir=cache_dir,
    )
    try:
        local_path = hf_hub_download(**args, local_files_only=True)
    except FileNotFoundError:
        local_path = hf_hub_download(**args, local_files_only=False)
    local_path = Path(local_path)

    logging.info(f'load {local_path}')
    if local_path.suffix == ".json":
        return json.loads(local_path.read_text())
    else:
        return torch.load(local_path)


def quantize(module: nn.Module, bits: int, groupsize: int, device: str,
             layer_types: Tuple[Any] = (nn.Conv2d, nn.Linear), prefix: str = ""):
    if isinstance(module, QuantLinear):
        return
    for name in dir(module):
        layer = getattr(module, name)
        layer_name = prefix + "." + name if prefix != "" else name
        if isinstance(layer, layer_types) and layer_name not in ["lm_head"]:
            delattr(module, name)
            quant_layer = QuantLinear(
                bits, groupsize,
                layer.in_features, layer.out_features,
                layer.bias is not None)
            setattr(module, name, quant_layer.to(device))
    for name, child in module.named_children():
        quantize(child, bits, groupsize, device, layer_types,
                 prefix + "." + name if prefix != "" else name)


class GPTQBigCodeModel(nn.Module):

    def __init__(self, model_name: str, bits: int, device: str, cache_dir: Optional[str]):
        super().__init__()

        if device == "cpu":
            raise ValueError("model is not implemented on cpu")

        self.encoding = SMCEncoding("bigcode_largemodel")
        self.device = device
        disable_torch_init()

        config = GPTBigCodeConfig.from_dict(load_filename("config.json", model_name, cache_dir))
        model = GPTBigCodeForCausalLM(config)
        model.eval()

        quantize(model, bits, groupsize=128, device=self.device)
        for name in ["transformer.wte.weight", "transformer.wpe.weight",
                     "transformer.ln_f.weight", "transformer.ln_f.bias", "lm_head.weight"] + \
                    [f"transformer.h.{idx}" for idx in range(len(model.transformer.h))]:
            model.load_state_dict({
                pname: weights.to(self.device)
                for pname, weights in load_filename(name, model_name, cache_dir).items()
            }, strict=False)
        self._model = model.to(self.device)
        self.cache_dir: Optional[str] = cache_dir
        self.model_name: str = model_name

    def forward(self, x, past_key_values: Optional = None, **unused):
        if past_key_values:
            past_key_values = [t[0] for t in past_key_values]
        output = self._model(x, past_key_values=past_key_values)
        return output.logits, [(t, ) for t in output.past_key_values]

    def lm_forward(self, x, **unused):
        return x  # inference is done in the `forward` method
