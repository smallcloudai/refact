import json
import os
from contextlib import asynccontextmanager
import asyncio

from refact_self_hosting import env


async def collect_longthinks(user2gpu_queue):
    config_file = os.path.join(env.DIR_CONFIG, "openai.json")
    model_name = 'longthink/stable'
    while True:
        try:
            if os.path.exists(config_file):
                with open(config_file, 'rb') as f:
                    config = json.load(f)
                if not config.get('is_enabled', True) or len(config.get('api_key', '')) == 0:
                    q = user2gpu_queue.pop(model_name, None)
                    while q is not None and not q.empty():
                        q.get_nowait()
                        q.task_done()
        except Exception as _:
            ...

        await asyncio.sleep(1)


# @asynccontextmanager
def collector(user2gpu_queue):
    asyncio.create_task(collect_longthinks(user2gpu_queue))
