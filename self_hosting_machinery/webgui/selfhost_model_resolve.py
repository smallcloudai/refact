import json

from self_hosting_machinery.webgui.selfhost_queue import InferenceQueue
from self_hosting_machinery import env
from refact_enterprise.known_models_db.refact_known_models import models_mini_db

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

    # find by hf name
    for k, dbrec in models_mini_db.items():
        if dbrec["model_path"] == model_name:
            model_name = k
            break

    # pass full model name
    if model_name in have_models:
        return model_name, ""

    return "", f"model is not loaded, available models: {have_models}"
