import os
import json
from refact_self_hosting.webgui import tab_models_host
from refact_self_hosting import env
from refact_self_hosting import enum_gpus


def copy_intact():
    lst = [
        "enum_gpus.cfg",
        "filetune.cfg",
        "process_uploaded.cfg",
        "webgui.cfg",
    ]
    for x in lst:
        dest = os.path.join(env.DIR_WATCHDOG_D, x)
        if os.path.exists(dest):
            continue
        os.symlink(
            os.path.join(env.DIR_WATCHDOG_TEMPLATES, x),
            dest,
        )


def copy_watchdog_configs_if_first_run_detected():
    if not os.path.exists(env.CONFIG_ENUM_GPUS):
        enum_gpus.enum_gpus()
        tab_models_host.first_run()
        copy_intact()


if __name__ == '__main__':
    copy_watchdog_configs_if_first_run_detected()
