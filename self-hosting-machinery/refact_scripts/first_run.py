import os
import json
from refact_webgui import tab_models_host
from refact_scripts import env, enum_gpus


def copy_watchdog_configs_if_first_run_detected():
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        tab_models_host.first_run()


if __name__ == '__main__':
    copy_watchdog_configs_if_first_run_detected()
