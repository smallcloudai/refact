import asyncio
import json
import os

from self_hosting_machinery.refact_scripts import env


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
    asyncio.create_task(collect_longthinks(user2gpu_queue))
