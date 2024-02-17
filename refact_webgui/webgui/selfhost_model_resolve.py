import json

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_queue import InferenceQueue

from typing import Tuple, List


def completion_resolve_model(inference_queue: InferenceQueue) -> Tuple[str, str]:
    have_models: List[str] = inference_queue.models_available()

    with open(env.CONFIG_INFERENCE, 'r') as f:
        completion_model = json.load(f).get("completion", None)

    if completion_model is None:
        return "", f"completion model is not set"

    if completion_model not in have_models:
        return "", f"model is not loaded (1)"

    return completion_model, ""


def static_resolve_model(model_name: str, inference_queue: InferenceQueue) -> Tuple[str, str]:
    # special case for longthink
    if model_name in ["longthink", "gpt3.5", "gpt4"]:
        model_name = "longthink/stable"
        if model_name not in inference_queue.models_available():
            return "", f"model is not loaded (2)"
        return model_name, ""

    have_models: List[str] = [
        model for model in inference_queue.models_available()
        if model not in ["longthink/stable"]
    ]

    # pass full model name
    if model_name in have_models:
        return model_name, ""

    # CONTRASTcode is default model
    if model_name in ["CONTRASTcode"]:
        model_name = ""

    def _family(model: str) -> str:
        return model.split("/")[0]

    for have_model in have_models:
        if not model_name or _family(model_name) == _family(have_model):
            return have_model, ""
    else:
        return "", f"model \"{model_name}\" is not loaded (3)"
