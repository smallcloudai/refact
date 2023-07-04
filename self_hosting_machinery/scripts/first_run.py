import os

from self_hosting_machinery.webgui import tab_models_host
from self_hosting_machinery.scripts import enum_gpus
from self_hosting_machinery import env


def copy_watchdog_configs_if_first_run_detected():
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        tab_models_host.first_run()


if __name__ == '__main__':
    copy_watchdog_configs_if_first_run_detected()
