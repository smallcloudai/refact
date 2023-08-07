import re
import os
import json
from typing import Dict

from self_hosting_machinery import env
from refact_data_pipeline.finetune.finetune_utils import default_finetune_model


def get_run_model_name(run_dir: str) -> str:
    config_json_fn = os.path.join(run_dir, "config.json")
    if not os.path.isfile(config_json_fn):
        raise RuntimeError("get run model name: no config.json found")
    with open(config_json_fn) as f:
        return json.load(f).get("model_name", default_finetune_model)


def find_best_lora(model_name: str) -> Dict[str, str]:
    error = "no completed runs found"
    for run_id in sorted(os.listdir(env.DIR_LORAS), reverse=True):
        # starts with latest
        run_dir = os.path.join(env.DIR_LORAS, run_id)
        if not os.path.isdir(run_dir):
            continue
        checkpoints_dir = os.path.join(run_dir, "checkpoints")
        if not os.path.isdir(checkpoints_dir):
            continue
        status_json_fn = os.path.join(run_dir, "status.json")
        if not os.path.isfile(status_json_fn):
            continue
        try:
            if get_run_model_name(run_dir) != model_name:
                continue
        except RuntimeError:
            continue
        error = "a completed run found, but there are no valid checkpoints"
        best_test_loss = 13
        best_checkpoint_dir = ""
        best_run_id = run_id
        best_checkpoint_id = ""
        for checkpoint_id in sorted(os.listdir(checkpoints_dir)):
            checkpoint_dir = os.path.join(checkpoints_dir, checkpoint_id)
            if not os.path.isdir(checkpoint_dir):
                continue
            with open(os.path.join(status_json_fn)) as f:
                status_json = json.load(f)
            if status_json["status"] not in ["completed", "finished"]:
                continue
            # iter0190-testloss0.678
            m = re.match(r"iter(\d+)-testloss(\d+\.\d+)", checkpoint_id)
            if m is None:
                continue
            iteration = int(m.group(1))
            test_loss = float(m.group(2))
            if test_loss < best_test_loss:
                best_test_loss = test_loss
                best_checkpoint_dir = checkpoint_dir
                best_checkpoint_id = checkpoint_id
        # if any checkpoint is good, return it
        if best_checkpoint_dir:
            return {
                "latest_run_id": best_run_id,
                "best_checkpoint_id": best_checkpoint_id,
                "path": best_checkpoint_dir,
                "error": "",
            }
        # possible problem: best in the recent run might be worse then in the previous
        # (when recent run dies for some reason)
    return {
        "latest_run_id": "",
        "best_checkpoint_id": "",
        "path": "",
        "error": error,
    }


if __name__ == "__main__":
    print(find_best_lora(default_finetune_model))
