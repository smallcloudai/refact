from dataclasses import dataclass
from self_hosting_machinery.webgui.selfhost_queue import InferenceQueue

from typing import Tuple, List


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
def resolve_model(model_name: str, inference_queue: InferenceQueue) -> Tuple[str, str]:
    """
    Allow client to specify less in the model string, including an empty string.
    """
    have_models: List[str] = inference_queue.models_available()
    if model_name in have_models:
        return model_name, ""

    if model_name in ["CONTRASTcode"]:
        model_name = ""
    if model_name in ["longthink", "gpt3.5", "gpt4"]:
        model_name = "longthink/stable"

    to_resolve = Model(model_name)
    if not to_resolve:
        filtered_hosted_models = [
            m for m in map(Model, have_models)
            if m.family not in ["longthink"]
        ]
    else:
        filtered_hosted_models = [
            m for m in map(Model, have_models)
            if m.family == to_resolve.family
        ]

    if not filtered_hosted_models:
        return "", f"model is not loaded"
    return str(filtered_hosted_models[0]), ""
