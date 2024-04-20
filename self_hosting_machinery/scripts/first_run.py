import os

from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from self_hosting_machinery.scripts import enum_gpus
from refact_utils.scripts import env


def assign_gpus_if_first_run_detected(model_assigner: ModelAssigner):
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        model_assigner.first_run()   # has models_to_watchdog_configs() inside


def convert_old_configs():
    # longthink.cfg and openai_api_worker.cfg are deprecated watchdog configs
    old_longthink = os.path.join(env.DIR_WATCHDOG_D, "longthink.cfg")
    if os.path.exists(old_longthink):
        os.unlink(old_longthink)
    openai_watchdog_cfg_fn = os.path.join(env.DIR_WATCHDOG_D, "openai_api_worker.cfg")
    if os.path.exists(openai_watchdog_cfg_fn):
        os.unlink(openai_watchdog_cfg_fn)


if __name__ == '__main__':
    convert_old_configs()
    model_assigner = ModelAssigner()
    assign_gpus_if_first_run_detected(model_assigner)
