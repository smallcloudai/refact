import re
import os
from refact_self_hosting import env


def find_best_lora(model_name: str) -> str:
    for run_id in sorted(os.listdir(env.DIR_LORAS), reverse=True):
        # starts with latest
        run_dir = os.path.join(env.DIR_LORAS, run_id)
        if not os.path.isdir(run_dir):
            continue
        checkpoints_dir = os.path.join(run_dir, "checkpoints")
        best_test_loss = 13
        best_checkpoint_dir = ""
        if not os.path.isdir(checkpoints_dir):
            continue
        for checkpoint_id in sorted(os.listdir(checkpoints_dir)):
            checkpoint_dir = os.path.join(checkpoints_dir, checkpoint_id)
            if not os.path.isdir(checkpoint_dir):
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
        # if any checkpoint is good, return it
        if best_checkpoint_dir:
            return best_checkpoint_dir
        # possible problem: best in the recent run might be worse then in the previous
        # (when recent run dies for some reason)
    return ""


if __name__ == "__main__":
    print(find_best_lora("CONTRASTcode/3b/multi", verbose=True))
