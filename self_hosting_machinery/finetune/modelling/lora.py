import math
from typing import Optional, List, Union, Dict

import torch as th
from torch.nn.init import kaiming_uniform_

__all__ = ['LoraMixin']


def inject_to_module(
        base_module: th.nn.Module,
        injecting_module: th.nn.Module,
        inject_path: str
):
    sub_paths = inject_path.split('.')
    for p in sub_paths[:-1]:
        if p.isnumeric():
            base_module = base_module[int(p)]
        else:
            base_module = getattr(base_module, p)

    if sub_paths[-1].isnumeric():
        base_module[int(sub_paths[-1])] = injecting_module
    else:
        setattr(base_module, sub_paths[-1], injecting_module)


class LoraLayerMixin:
    def __init__(
            self,
            r: int,
            lora_alpha: int,
            lora_dropout: float,
    ):
        self.r = r
        self.lora_alpha = lora_alpha
        # Optional dropout
        if lora_dropout > 0.0:
            self.lora_dropout = th.nn.Dropout(p=lora_dropout)
        else:
            self.lora_dropout = lambda x: x


class LoraLinear(LoraLayerMixin, th.nn.Module):
    def __init__(
            self,
            layer: th.nn.Module,
            r: int = 0,
            lora_alpha: int = 1,
            lora_dropout: float = 0.0,
            init_scale: float = 0.0,
            dummy_output: bool = False,
            **unused
    ):
        th.nn.Module.__init__(self)
        LoraLayerMixin.__init__(self, r=r, lora_alpha=lora_alpha, lora_dropout=lora_dropout)
        self.layer = layer
        self.layer.weight.requires_grad = False
        if self.layer.bias is not None:
            self.layer.bias.requires_grad = False
        self.init_scale = init_scale
        self.dummy_output = dummy_output

        self.out_features, self.in_features = self.layer.weight.shape

        # Actual trainable parameters
        if r > 0:
            self.lora_A = th.nn.Linear(self.in_features, r, bias=False,
                                       device=self.layer.weight.device, dtype=self.layer.weight.dtype)
            self.lora_B = th.nn.Linear(r, self.out_features, bias=False,
                                       device=self.layer.weight.device, dtype=self.layer.weight.dtype)
            self.scaling = self.lora_alpha / self.r

        self.init_weights()

    def init_weights(self):
        if self.r > 0:
            # initialize A the same way as the default for th.nn.Linear and B to zero
            kaiming_uniform_(self.lora_A.weight, a=math.sqrt(5))
            self.lora_A.weight.data *= self.init_scale / self.lora_A.weight.data.norm(dim=1, p=2, keepdim=True)
            th.nn.init.zeros_(self.lora_B.weight)

    @property
    def weight(self) -> th.nn.Parameter:
        return self.layer.weight + (self.lora_B.weight @ self.lora_A.weight) * self.scaling

    @property
    def bias(self) -> Optional[th.nn.Parameter]:
        return self.layer.bias

    def forward(self, x: th.Tensor) -> Union[th.Tensor, List[th.Tensor]]:
        if self.r > 0:
            if not self.dummy_output:
                result1 = self.layer(x)
            else:
                result1, *other = self.layer(x)
            result2 = result1 + self.lora_B(self.lora_A(self.lora_dropout(x))) * self.scaling
            if not self.dummy_output:
                return result2
            else:
                return result2, *other
        else:
            return self.layer(x)


class LoraMixin:
    @staticmethod
    def apply_lora(
            model: th.nn.Module,
            lora_target_modules: List[str],
            lora_r: int,
            lora_alpha: int,
            lora_dropout: float,
            lora_init_scale: float = 0.01,
            **unused
    ):
        # TODO: compat fix, remove in the next iteration of changes
        lora_target_modules = ['out' if m == 'backproj' else m for m in lora_target_modules]
        for name, module in model.named_modules():
            if not isinstance(module, th.nn.Linear):
                continue
            if any(m in name for m in lora_target_modules):
                new_module = LoraLinear(
                    module, lora_r, lora_alpha, lora_dropout, init_scale=lora_init_scale
                )
                inject_to_module(
                    base_module=model,
                    injecting_module=new_module,
                    inject_path=name
                )

    @staticmethod
    def exclude_lora(model: th.nn.Module):
        for name, module in model.named_modules():
            if not isinstance(module, LoraLinear):
                continue
            inject_to_module(
                base_module=model,
                injecting_module=module.layer,
                inject_path=name
            )

    @staticmethod
    def lora_merged_state_dict(model: th.nn.Module) -> Dict[str, th.Tensor]:
        state_dict = model.state_dict()
        for name, module in model.named_modules():
            if not isinstance(module, LoraLinear):
                continue
            weight, bias = module.weight, module.bias
            for lora_name in (f"{name}.layer.weight",
                              f"{name}.lora_A.weight",
                              f"{name}.lora_B.weight"):
                state_dict.pop(lora_name)
            state_dict[f"{name}.weight"] = weight
            if bias is not None:
                state_dict[f"{name}.bias"] = bias
        return state_dict
