import traceback
import logging

from refact_self_hosting import env
import subprocess
import xml.etree.ElementTree as ET
import json


def run_nvidia_smi():
    j = {
        "gpus": []
    }
    txt = "- no output -"
    try:
        txt = subprocess.check_output(['nvidia-smi', '-q', '-x'])
        root = ET.fromstring(txt)
        for gpu in root:
            if gpu.tag == 'gpu':
                gpu_id = gpu.attrib['id']
                gpu_name = "unknown"
                gpu_mem_total = 0
                gpu_mem_used = 0
                gpu_temp = 0
                for child in gpu:
                    if child.tag == 'product_name':
                        gpu_name = child.text
                    if child.tag == 'fb_memory_usage':
                        for child2 in child:
                            if child2.tag == 'total':
                                gpu_mem_total = child2.text
                            if child2.tag == 'used':
                                gpu_mem_used = child2.text
                    if child.tag == 'temperature':
                        for child2 in child:
                            if child2.tag == 'gpu_temp':
                                gpu_temp = child2.text
                # print(gpu_id, gpu_name, "mem", gpu_mem_used, gpu_mem_total, gpu_temp)
                j["gpus"].append({
                    "id": gpu_id,
                    "name": gpu_name,
                    "mem_used_mb": int(gpu_mem_used.split()[0]),
                    "mem_total_mb": int(gpu_mem_total.split()[0]),
                    "temp_celsius": int(gpu_temp.split()[0])
                })
    except Exception:
        logging.warning("nvidia-smi does not work, that's especially bad for initial setup.")
        logging.warning(traceback.format_exc())
        logging.warning("output was:\n%s" % txt)

    return j


def enum_gpus():
    result = run_nvidia_smi()
    with open(env.CONFIG_ENUM_GPUS, 'w') as f:
        json.dump(result, f, indent=4)


if __name__ == '__main__':
    enum_gpus()
