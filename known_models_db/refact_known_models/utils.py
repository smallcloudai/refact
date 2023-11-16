from dataclasses_json import dataclass_json
from dataclasses import dataclass
from dataclasses import field

from typing import Any, Dict, List, Optional, Iterable, Set, Tuple


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
    default_finetune: bool = False

    @property
    def family(self) -> str:
        return self.name.split("/")[0]

    @staticmethod
    def __unique_id(spec: Dict) -> Tuple:
        return (
            spec.get("name", None),
            spec.get("model_path", None),
            spec.get("quantization", None),
        )

    def __hash__(self) -> int:
        return hash(self.__unique_id(self.to_dict()))

    def __eq__(self, other) -> bool:
        if isinstance(other, dict):
            return hash(self) == hash(self.__unique_id(other))
        elif isinstance(other, ModelSpec):
            return hash(self) == hash(other)
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

        # duplicate specs validation
        validated_specs = set()
        for spec in self._specs:
            assert spec not in validated_specs, f"duplicate spec: {spec}"
            validated_specs.add(spec)

        # default spec validation
        for model_name in {spec.name for spec in self._specs}:
            default_specs = [
                spec for spec in self._specs
                if spec.name == model_name and spec.default
            ]
            assert default_specs, f"default spec for model '{model_name}' not found"
            assert len(default_specs) == 1, f"multiple default specs for model '{model_name}'"

            finetune_specs = [
                spec for spec in self._specs
                if spec.name == model_name and spec.finetune
            ]
            finetune_default_specs = [
                spec for spec in self._specs
                if spec.name == model_name and spec.default_finetune
            ]
            if finetune_specs:
                assert finetune_default_specs, f"default finetune spec for model '{model_name}' not found"
                assert len(finetune_default_specs) == 1, f"multiple default finetune specs for model '{model_name}'"

        # validate context sizes
        for spec in self._specs:
            assert len(spec.context_sizes), f"no context sizes for model '{spec.name}'"

    @property
    def models(self) -> Set[str]:
        return {spec.name for spec in self._specs}

    def find_spec(self, spec: Dict) -> Optional[ModelSpec]:
        specs = [s for s in self._specs if s == spec]
        return specs[0] if specs else None

    def default(self, model_name: str) -> ModelSpec:
        default_specs = [
            spec for spec in self._specs
            if spec.name == model_name and spec.default
        ]
        return default_specs[0]

    def default_finetune(self, model_name: str) -> Optional[ModelSpec]:
        default_specs = [
            spec for spec in self._specs
            if spec.name == model_name and spec.default_finetune
        ]
        return default_specs[0] if default_specs else None

    @property
    def specs(self) -> List[ModelSpec]:
        return self._specs
