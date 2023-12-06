import os

from self_hosting_machinery.webgui.selfhost_model_assigner import ModelAssigner
from self_hosting_machinery.scripts import enum_gpus
from self_hosting_machinery import env


def copy_watchdog_configs_if_first_run_detected(model_assigner: ModelAssigner):
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        model_assigner.first_run()


def convert_old_configs(model_assigner: ModelAssigner):
    # longthink.cfg and openai_api_worker.cfg are deprecated watchdog configs
    old_longthink = os.path.join(env.DIR_WATCHDOG_D, "longthink.cfg")
    if os.path.exists(old_longthink):
        os.unlink(old_longthink)
    openai_watchdog_cfg_fn = os.path.join(env.DIR_WATCHDOG_D, "openai_api_worker.cfg")
    if os.path.exists(openai_watchdog_cfg_fn):
        os.unlink(openai_watchdog_cfg_fn)

    for gpu in range(16):
        fn = os.path.join(env.DIR_WATCHDOG_D, "model-gpu%d.cfg" % gpu)
        if not os.path.exists(fn):
            continue
        text = open(fn).read()

    model_assigner.models_to_watchdog_configs()


if __name__ == '__main__':
    model_assigner = ModelAssigner()
    convert_old_configs(model_assigner)
    copy_watchdog_configs_if_first_run_detected(model_assigner)
