from dataclasses_json import dataclass_json
from dataclasses import dataclass
from dataclasses import field

from typing import Any, Dict, List, Optional, Iterable, Set


@dataclass_json
@dataclass
class ModelSpec:
    name: str

    context_sizes: List[int]
    filter_caps: List[str]
    diff_scratchpad_class: Optional[str]
    chat_scratchpad_class: Optional[str]

    backend: str
    model_path: str
    quantization: Optional[str] = None
    model_class_kwargs: Dict[str, Any] = field(default_factory=dict)
    completion: bool = False
    finetune: bool = False
    default: bool = False

    @property
    def family(self) -> str:
        return self.name.split("/")[0]

    def __eq__(self, other):
        if isinstance(other, dict):
            return self.name == other.get("name", None) and \
                self.model_path == other.get("model_path", None) and \
                self.quantization == other.get("quantization", None)
        elif isinstance(other, ModelSpec):
            return self.name == other.name and \
                self.model_path == other.model_path and \
                self.quantization == other.quantization
        assert False, f"cannot compare ModelSpec with {type(other)}"


def model_specs_from_list(
        name: str,
        context_sizes: List[int],
        specs_kwargs: List[Dict[str, Any]],
        completion: bool = False,
        filter_caps: List[str] = [],
        diff_scratchpad_class: Optional[str] = None,
        chat_scratchpad_class: Optional[str] = None) -> Iterable[ModelSpec]:
    for spec_kwargs in specs_kwargs:
        yield ModelSpec(
            name=name, context_sizes=context_sizes, filter_caps=filter_caps, completion=completion,
            diff_scratchpad_class=diff_scratchpad_class, chat_scratchpad_class=chat_scratchpad_class,
            **spec_kwargs)


class ModelRegistry:

    def __init__(self, specs: Iterable[ModelSpec]):
        self._specs: List[ModelSpec] = list(specs)

    @property
    def models(self) -> Set[str]:
        return {spec.name for spec in self._specs}

    def find_spec(self, spec: Dict) -> Optional[ModelSpec]:
        specs = [s for s in self._specs if s == spec]
        assert len(specs) <= 1, f"multiple specs match {spec}"
        return specs[0] if specs else None

    def default(self, model_name: str) -> ModelSpec:
        default_specs = [
            spec for spec in self._specs
            if spec.name == model_name and spec.default
        ]
        if not default_specs:
            assert False, f"default spec for model '{model_name}' not found"
        elif len(default_specs) > 1:
            assert False, f"multiple default specs for model '{model_name}'"
        return default_specs[0]

    @property
    def specs(self) -> List[ModelSpec]:
        return self._specs
