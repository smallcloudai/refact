import os

from self_hosting_machinery.webgui import tab_models_host
from self_hosting_machinery.scripts import enum_gpus
from self_hosting_machinery import env


def copy_watchdog_configs_if_first_run_detected():
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        tab_models_host.first_run()


def convert_old_configs():
    # longthink.cfg is an old version of openai_api_worker.cfg
    force_models_cfg_update = False
    old_longthink = os.path.join(env.DIR_WATCHDOG_D, "longthink.cfg")
    if os.path.exists(old_longthink):
        os.unlink(old_longthink)
        force_models_cfg_update = True

    for gpu in range(16):
        fn = os.path.join(env.DIR_WATCHDOG_D, "model-gpu%d.cfg" % gpu)
        if not os.path.exists(fn):
            continue
        text = open(fn).read()
        if "refact_self_hosting.inference_worker" in text:
            force_models_cfg_update = True

    if force_models_cfg_update:
        tab_models_host.models_to_watchdog_configs()



if __name__ == '__main__':
    convert_old_configs()
    copy_watchdog_configs_if_first_run_detected()
