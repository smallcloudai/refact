import asyncio
import json
import os

from self_hosting_machinery import env

from typing import Optional, List


async def collect_models(user2gpu_queue):
    def _get_model_name(args: List[str]) -> Optional[str]:
        try:
            index = args.index("--model") + 1
            return args[index]
        except ValueError:
            return None
        except IndexError:
            return None

    while True:
        enabled_models = []
        for config_file in filter(
                lambda name: name.startswith("model"),
                os.listdir(env.DIR_WATCHDOG_D)):
            config_file = os.path.join(env.DIR_WATCHDOG_D, config_file)
            if os.path.exists(config_file):
                with open(config_file, 'r') as f:
                    config = json.load(f)
                model_name = _get_model_name(config["command_line"])
                if model_name is not None:
                    enabled_models.append(model_name)
        for model_name in set(user2gpu_queue.keys()).difference(enabled_models):
            q = user2gpu_queue.pop(model_name, None)
            while q is not None and not q.empty():
                q.get_nowait()
                q.task_done()

        await asyncio.sleep(1)


async def collect_longthinks(user2gpu_queue):
    config_file = env.CHATGPT_CONFIG_FILENAME
    model_name = 'longthink/stable'
    while True:
        if os.path.exists(config_file):
            with open(config_file, 'r') as f:
                config = json.load(f)
            if not config.get('is_enabled', True):
                q = user2gpu_queue.pop(model_name, None)
                while q is not None and not q.empty():
                    q.get_nowait()
                    q.task_done()

        await asyncio.sleep(1)


def collector(user2gpu_queue):
    asyncio.create_task(collect_models(user2gpu_queue))
    asyncio.create_task(collect_longthinks(user2gpu_queue))
