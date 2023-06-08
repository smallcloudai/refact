import json
import traceback
import logging
import subprocess

from refact_self_hosting import env


def query_nvidia_smi():
    nvidia_smi_output = "- no output -"
    descriptions = []
    try:
        nvidia_smi_output = subprocess.check_output([
            "nvidia-smi",
            "--query-gpu", "pci.bus_id,name,memory.used,memory.total,temperature.gpu",
            "--format", "csv"])
        for description in nvidia_smi_output.decode().splitlines()[1:]:
            gpu_bus_id, gpu_name, gpu_mem_used, gpu_mem_total, gpu_temp = description.split(", ")
            descriptions.append({
                "id": gpu_bus_id,
                "name": gpu_name,
                "mem_used_mb": int(gpu_mem_used.split()[0]),
                "mem_total_mb": int(gpu_mem_total.split()[0]),
                "temp_celsius": int(gpu_temp.split()[0])
            })
    except Exception:
        logging.warning("nvidia-smi does not work, that's especially bad for initial setup.")
        logging.warning(traceback.format_exc())
        logging.warning(f"output was:\n{nvidia_smi_output}")

    return {"gpus": descriptions}


def enum_gpus():
    result = query_nvidia_smi()
    with open(env.CONFIG_ENUM_GPUS, 'w') as f:
        json.dump(result, f, indent=4)


if __name__ == '__main__':
    enum_gpus()
