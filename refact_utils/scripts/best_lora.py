import re
import os
import json

from refact_utils.scripts import env
from self_hosting_machinery.finetune.utils.finetune_utils import get_run_model_name  # REFACTORME
from self_hosting_machinery.finetune.utils.finetune_utils import default_finetune_model  # REFACTORME

from typing import Dict, Optional


def find_best_checkpoint(run_id: str) -> Dict[str, str]:
    run_dir = os.path.join(env.DIR_LORAS, run_id)
    if not os.path.isdir(run_dir):
        raise RuntimeError(f"run_id not found")
    checkpoints_dir = os.path.join(run_dir, "checkpoints")
    if not os.path.isdir(checkpoints_dir):
        raise RuntimeError(f"run_id has no checkpoints")

    def checkpoint_name_to_loss(checkpoint_id: str) -> Optional[float]:
        match = re.match(r"iter(\d+)-testloss(\d+\.\d+)", checkpoint_id)
        if match is None:
            return None
        return float(match.group(2))

    checkpoints = list(filter(lambda x: x[0] is not None and os.path.isdir(x[1]), [
        (
            checkpoint_name_to_loss(checkpoint_id),
            os.path.join(checkpoints_dir, checkpoint_id),
            checkpoint_id,
        )
        for checkpoint_id in os.listdir(checkpoints_dir)
    ]))

    if not checkpoints:
        raise RuntimeError(f"run_id has no valid checkpoints")

    best_checkpoint = min(checkpoints, key=lambda x: x[0])
    return {
        "best_checkpoint_id": best_checkpoint[2],
        "path": best_checkpoint[1],
    }


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
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--model", type=str, default=default_finetune_model)
    args = parser.parse_args()

    best_lora = find_best_lora(args.model)
    try:
        best_checkpoint = find_best_checkpoint(best_lora["latest_run_id"])
    except RuntimeError as e:
        best_checkpoint = None
    print("Best LoRA", best_lora)
    print("Best checkpoint", best_checkpoint)
