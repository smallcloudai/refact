import os
import sys
import signal
import asyncio
from typing import Iterable
from pathlib import Path

import psutil
import ujson as json

from self_hosting_machinery.scripts import env
from refact_vecdb import VecDBAsyncAPI


async def upload_vecdb_paths(paths: Iterable[Path]):
    vecdb_api = VecDBAsyncAPI()
    await vecdb_api.delete_all_records()
    await vecdb_api.upload_files(paths)


def catch_sigusr1(signum, frame):
    print("catched SIGUSR1")
    current_process = psutil.Process()
    for child in current_process.children(recursive=False):
        os.kill(child.pid, signal.SIGUSR1)


def main():
    signal.signal(signal.SIGUSR1, catch_sigusr1)

    try:
        print('vecdb_upload: starting')
        unpacked_dir = Path(env.DIR_UNPACKED)
        assert unpacked_dir.is_dir(), f'{unpacked_dir} is not a directory'

        paths_upload_file = unpacked_dir / 'vecdb_paths_upload.json'
        assert paths_upload_file.is_file(), f'{paths_upload_file} is not a file or does not exist'

        paths_upload = json.loads(paths_upload_file.read_text())
        assert paths_upload, f'paths_upload is empty'

        asyncio.run(upload_vecdb_paths([Path(p) for p in paths_upload]))
    except Exception as e:
        print("vecdb_upload: %s" % e)
        sys.exit(1)


if __name__ == "__main__":
    main()

