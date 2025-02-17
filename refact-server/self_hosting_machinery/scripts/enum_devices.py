import json
import os
import time
import signal
import sys
import psutil
import traceback
import logging
import subprocess

from refact_utils.scripts import env


def get_cpu_info():
    cpu_info = {
        "id": "cpu",
        "name": "CPU",  # NOTE: do we really need the actual name of processor?
        "mem_used_mb": 0,
        "mem_total_mb": 1,
        "temp_celsius": -1,
    }
    try:
        mem = psutil.virtual_memory()
        temps = [t.current for t in psutil.sensors_temperatures().get("coretemp", [])]
        cpu_info["mem_used_mb"] = mem.used // (1 << 20)
        cpu_info["mem_total_mb"] = mem.total // (1 << 20)
        cpu_info["temp_celsius"] = int(sum(temps) / len(temps)) if temps else -1
    except Exception:
        logging.warning("psutil can't get info about CPU")
        logging.warning(traceback.format_exc())
    return cpu_info


def query_nvidia_smi():
    nvidia_smi_output = "- no output -"
    gpu_infos = []
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
            gpu_infos.append({
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

    return gpu_infos


def enum_gpus():
    result = {
        "cpu": get_cpu_info(),
        "gpus": query_nvidia_smi(),
    }
    with open(env.CONFIG_ENUM_DEVICES + ".tmp", 'w') as f:
        json.dump(result, f, indent=4)
    os.rename(env.CONFIG_ENUM_DEVICES + ".tmp", env.CONFIG_ENUM_DEVICES)


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
