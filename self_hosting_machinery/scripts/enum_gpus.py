import json
import os
import time
import signal
import sys
import traceback
import logging
import subprocess

from self_hosting_machinery import env

def query_rocm_smi():
    rocm_smi_output = "- no output -"
    descriptions = []
    try:
        rocm_smi_output = subprocess.check_output([
            "/opt/rocm/bin/rocm-smi", 
            "--showbus", 
            "--showproductname", 
            "--showtemp",
            "--showmeminfo", "vram",
            "--json"])
        logging.info(rocm_smi_output)
        smi_output_dict = json.loads(rocm_smi_output)
        for gpu_id, props in smi_output_dict.items():
            descriptions.append({
                "id": props.get("PCI Bus"),
                "name": props.get("Card model", "AMD GPU"),
                "mem_used_mb": bytes_to_mb(int(props.get("VRAM Total Used Memory (B)", 0))),
                "mem_total_mb": bytes_to_mb(int(props.get("VRAM Total Memory (B)", 0 ))),
                "temp_celsius": props.get("Temperature (Sensor junction) (C)", -1),
            })
    except Exception:
        logging.warning("rocm-smi does not work, that's especially bad for initial setup.")
        logging.warning(traceback.format_exc())
        logging.warning(f"output was:\n{smi_output_dict}")

    return {"gpus": descriptions}

def bytes_to_mb(bytes_size):
    mb_size = bytes_size / (1024 ** 2)
    return mb_size


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
        logging.warning("nvidia-smi does not work, that's especially bad for initial setup.")
        logging.warning(traceback.format_exc())
        logging.warning(f"output was:\n{nvidia_smi_output}")

    return {"gpus": descriptions}


def enum_gpus():
    if os.environ.get('USE_ROCM'):
        result = query_rocm_smi()
    else:
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
