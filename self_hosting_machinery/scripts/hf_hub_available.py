import os
import subprocess

from refact_utils.scripts import env
from huggingface_hub.constants import ENDPOINT
from urllib.parse import urlparse


def _is_hf_available(timeout: float) -> bool:
    try:
        retval = subprocess.call(
            ["ping", "-c", "1", urlparse(ENDPOINT).hostname],
            stderr=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            timeout=timeout,
        )
        return retval == 0
    except:
        return False


def set_hf_hub_offline_flag():
    if _is_hf_available(timeout=5):
        if os.path.exists(env.FLAG_HF_HUB_OFFLINE):
            os.unlink(env.FLAG_HF_HUB_OFFLINE)
    else:
        with open(env.FLAG_HF_HUB_OFFLINE, "w"):
            pass


if __name__ == "__main__":
    set_hf_hub_offline_flag()
