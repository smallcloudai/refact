from dataclasses import dataclass

from typing import Tuple, Iterable


@dataclass
class Model:
    family: str
    size: str = ""
    specialization: str = ""
    version: str = ""

    def __init__(self, name: str):
        self.family, self.size, self.specialization, self.version = \
            f"{name}///".split("/")[:4]

    def __str__(self) -> str:
        return "/".join([
            self.family,
            self.size,
            self.specialization,
            self.version
        ]).rstrip("/")

    def __bool__(self) -> bool:
        return bool(self.family)


# TODO: remove this function ASAP, we need dynamic resolve mechanism
def resolve_model(model_name: str, hosted_models: Iterable[str]) -> Tuple[str, str]:
    """
    Allow client to specify less in the model string, including an empty string.
    """
    if model_name in hosted_models:
        return model_name, ""

    if model_name in ["CONTRASTcode"]:
        model_name = ""
    if model_name in ["longthink", "gpt3.5", "gpt4"]:
        model_name = "longthink/stable"

    to_resolve = Model(model_name)
    if not to_resolve:
        filtered_hosted_models = [
            m for m in map(Model, hosted_models)
            if m.family not in ["longthink"]
        ]
    else:
        filtered_hosted_models = [
            m for m in map(Model, hosted_models)
            if m.family == to_resolve.family
        ]

    if not filtered_hosted_models:
        return "", f"no loaded model of family '{to_resolve.family}'"
    return str(filtered_hosted_models[0]), ""
