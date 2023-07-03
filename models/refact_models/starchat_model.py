from typing import Optional

from torch import nn
from transformers import AutoModelForCausalLM

from code_contrast import SMCEncoding


class StarChatModel(nn.Module):
    def __init__(self,
                 checkpoint: str, device: str,
                 cache_dir: Optional[str] = None):
        super().__init__()

        if device == "cpu":
            raise ValueError("model is not implemented on cpu")

        self.encoding = SMCEncoding("bigcode_starchat")
        self.device = device
        self.model = AutoModelForCausalLM.from_pretrained(
            checkpoint,
            cache_dir=cache_dir,
            trust_remote_code=True)

    @classmethod
    def from_pretrained(self,
                        path: str,
                        device: str = "cuda",
                        cache_dir: Optional[str] = None,
                        **unused):
        return StarChatModel(path, device, cache_dir=cache_dir)

    def forward(self, x, past_key_values: Optional = None, **unused):
        if past_key_values:
            past_key_values = [t[0] for t in past_key_values]
        output = self.model(x, past_key_values=past_key_values)
        return output.logits, [(t,) for t in output.past_key_values]

    def lm_forward(self, x, **unused):
        return x  # inference is done in the `forward` method

    def generate(self, inputs, **kwargs):
        return self.model.generate(inputs, **kwargs)
