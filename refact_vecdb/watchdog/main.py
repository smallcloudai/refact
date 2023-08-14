import os
import signal
import asyncio
import time
from typing import Iterable
from pathlib import Path

import psutil
import ujson as json

from self_hosting_machinery.scripts import env
from refact_vecdb import VecDBAsyncAPI


async def upload_vecdb_paths(paths: Iterable[Path]):
    vecdb_api = VecDBAsyncAPI()
    await vecdb_api.upload_files(paths)


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
    while True:
        try:
            if not db_set_file.is_file() or not db_set_meta_file.is_file():
                raise Exception('db_set_file or db_set_meta_file is not found')

            db_set = db_set_file.read_text()
            db_set_meta = json.loads(db_set_meta_file.read_text())

            if not db_set or not db_set_meta or not db_set_meta.get('modified_ts'):
                raise Exception('db_set or db_set_meta is empty')

            if not (to_process := db_set_meta.get('to_process')):
                raise Exception(f'to_process flag interrupted: {to_process}')

            if (modified_time := (time.time() - (db_set_meta.get('modified_ts')))) < 30:
                raise Exception(f'modified_ts is no older than 30 seconds: {int(modified_time)}s')

            paths_upload = [unpacked_dir / json.loads(line)['path'] for line in db_set.splitlines()]

            if not paths_upload:
                raise Exception('paths_upload is empty')

            asyncio.run(upload_vecdb_paths(paths_upload))

        except Exception as e:
            print("vecdb_upload: %s" % e)
        else:
            with db_set_meta_file.open('w') as f:
                f.write(json.dumps({'modified_ts': time.time(), 'to_process': False}))

        time.sleep(10)


if __name__ == "__main__":
    main()

