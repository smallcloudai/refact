from typing import Optional

import torch
from torch import nn
from transformers import AutoModelForCausalLM

from refact_encoding import RefactEncoding
from code_contrast.modeling.lora import LoraMixin


class HFModel(nn.Module, LoraMixin):

    def __init__(
            self,
            model_name: str,
            device: str,
            cache_dir: Optional[str] = None,
            use_auth_token: Optional[str] = None
    ):
        super().__init__()
        self.device = device
        self.model = AutoModelForCausalLM.from_pretrained(
            model_name,
            cache_dir=cache_dir,
            trust_remote_code=True,
            use_auth_token=use_auth_token,
        ).to(device)
        self.encoding = RefactEncoding(model_name.replace('/', '_'))
        self.cache_dir: Optional[str] = cache_dir
        self.model_name: str = model_name

    @classmethod
    def from_pretrained(self,
                        path: str,
                        device: str = "cuda",
                        cache_dir: Optional[str] = None,
                        **unused):
        return HFModel(path, device, cache_dir=cache_dir)

    def forward(self, x, past_key_values: Optional = None, **unused):
        output = self.model(x, past_key_values=past_key_values)
        return output.logits, output.past_key_values

    def lm_forward(self, x, **unused):
        return x  # inference is done in the `forward` method

    def to_device(self, module: nn.Module):
        module = module.to(self.device)
        if self.device.startswith("cuda"):
            module = module.to(torch.half)
        return module

    def generate(self, inputs):
        return self.model.generate(inputs)
