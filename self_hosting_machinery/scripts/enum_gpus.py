import json
import os
import time
import signal
import sys
import traceback
import subprocess

from self_hosting_machinery import env
from self_hosting_machinery.webgui.selfhost_webutils import logger


def query_nvidia_smi():
    nvidia_smi_output = "- no output -"
    descriptions = []
    try:
        nvidia_smi_output = subprocess.check_output([
            "nvidia-smi",
            "--query-gpu=pci.bus_id,name,memory.used,memory.total,temperature.gpu",
            "--format=csv"])
        for description in nvidia_smi_output.decode().splitlines()[1:]:
            gpu_bus_id, gpu_name, gpu_mem_used, gpu_mem_total, gpu_temp = description.split(", ")
            gpu_mem_used_mb = int(gpu_mem_used.split()[0])
            gpu_mem_total_mb = int(gpu_mem_total.split()[0])
            try:
                gpu_temp_celsius = int(gpu_temp.split()[0])
            except ValueError:
                gpu_temp_celsius = -1
            descriptions.append({
                "id": gpu_bus_id,
                "name": gpu_name,
                "mem_used_mb": gpu_mem_used_mb,
                "mem_total_mb": gpu_mem_total_mb,
                "temp_celsius": gpu_temp_celsius,
            })
    except Exception:
        logger.warning("nvidia-smi does not work, that's especially bad for initial setup.")
        logger.warning(traceback.format_exc())
        logger.warning(f"output was:\n{nvidia_smi_output}")

    return {"gpus": descriptions}


def enum_gpus():
    result = query_nvidia_smi()
    with open(env.CONFIG_ENUM_GPUS + ".tmp", 'w') as f:
        json.dump(result, f, indent=4)
    os.rename(env.CONFIG_ENUM_GPUS + ".tmp", env.CONFIG_ENUM_GPUS)


if __name__ == '__main__':
    quit_flag = False

    def catch_sigkill(signum, frame):
        sys.stderr.write("enum_gpus caught SIGUSR1\n")
        sys.stderr.flush()
        global quit_flag
        quit_flag = True

    signal.signal(signal.SIGUSR1, catch_sigkill)
    next_wakeup = time.time() + 2
    for _ in range(3600):
        next_wakeup += 2
        if quit_flag:
            break
        enum_gpus()
        time.sleep(max(0, next_wakeup - time.time()))
        # from datetime import datetime
        # print(datetime.utcnow().strftime("%H:%M:%S.%f"))
