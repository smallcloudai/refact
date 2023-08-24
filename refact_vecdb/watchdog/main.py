import os
import signal
import asyncio
import time
import traceback
from typing import Iterable, List, Optional
from pathlib import Path

import psutil
import ujson as json

from self_hosting_machinery.scripts import env
from refact_vecdb import VecDBAsyncAPI


async def upload_vecdb_paths(paths: Iterable[Path]):
    if not paths:
        return
    vecdb_api = VecDBAsyncAPI()
    await vecdb_api.upload_files(paths)


async def vecdb_update_provider(provider: str):
    vecdb_api = VecDBAsyncAPI()
    async for batch in vecdb_api.update_provider(provider):
        pass


async def get_all_file_names() -> List[Optional[str]]:
    vecdb_api = VecDBAsyncAPI()
    return await vecdb_api.get_all_file_names()


async def delete_deleted_files(ex_file_names: List[Path]):
    vecdb_api = VecDBAsyncAPI()
    vdb_file_names = await get_all_file_names()
    diff_names = set(vdb_file_names).difference(set(str(e) for e in ex_file_names))
    if not diff_names:
        return

    await vecdb_api.delete_files_by_name(diff_names)


def catch_sigusr1(signum, frame):
    print("catched SIGUSR1")
    current_process = psutil.Process()
    for child in current_process.children(recursive=False):
        os.kill(child.pid, signal.SIGUSR1)


def main():
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    unpacked_dir = Path(env.DIR_UNPACKED)
    db_set_file = unpacked_dir / 'database_set.jsonl'
    db_set_meta_file = unpacked_dir / 'database_set_meta.json'
    vecdb_update_provider_file = unpacked_dir / 'vecdb_update_provider.json'
    while True:
        try:
            if vecdb_update_provider_file.exists():
                provider = json.loads(vecdb_update_provider_file.read_text())['provider']
                vecdb_update_provider_file.unlink()
                asyncio.run(vecdb_update_provider(provider))

            if not db_set_file.is_file() or not db_set_meta_file.is_file():
                raise Exception('db_set_file or db_set_meta_file is not found')

            db_set = db_set_file.read_text()
            db_set_meta = json.loads(db_set_meta_file.read_text())

            if not db_set_meta or not db_set_meta.get('modified_ts'):
                raise Exception('db_set_meta is empty')

            if not (to_process := db_set_meta.get('to_process')):
                raise Exception(f'to_process flag interrupted: {to_process}')

            if (modified_time := (time.time() - (db_set_meta.get('modified_ts')))) < 5:
                raise Exception(f'modified_ts is no older than 5 seconds: {int(modified_time)}s')

            paths_upload = [unpacked_dir / json.loads(line)['path'] for line in db_set.splitlines()]

            asyncio.run(delete_deleted_files(paths_upload))
            asyncio.run(upload_vecdb_paths(paths_upload))

        except Exception as e:
            # traceback.print_exc()
            print(e)
        else:
            with db_set_meta_file.open('w') as f:
                f.write(json.dumps({'modified_ts': time.time(), 'to_process': False}))

        time.sleep(3)


if __name__ == "__main__":
    main()

