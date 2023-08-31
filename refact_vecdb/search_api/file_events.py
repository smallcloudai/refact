import traceback

from pathlib import Path

import ujson as json

from watchdog.events import FileSystemEventHandler

from refact_vecdb.common.profiles import VDBFiles
from refact_vecdb.search_api.vecdb import load_vecdb
from refact_vecdb.search_api.context import CONTEXT as C


__all__ = ['WorkDirEventsHandler']


class WorkDirEventsHandler(FileSystemEventHandler):
    def __init__(
            self,
            workdir: Path,
            keyspace: str
    ):
        self._workdir = workdir
        self._keyspace = keyspace
        self._update_indexes_file = self._workdir.joinpath(VDBFiles.update_indexes)
        self._update_indexes_file.unlink(missing_ok=True)

    def on_modified(self, event):
        try:
            if event.src_path.endswith(str(VDBFiles.update_indexes)):
                provider = json.loads(self._update_indexes_file.read_text())['provider']
                C.c_sessions[self._keyspace]['provider'] = provider
                print(f'detected update_indexes for keyspace: {self._keyspace}')
                load_vecdb(self._keyspace)
                self._update_indexes_file.unlink()
        except Exception as e:
            traceback.print_exc()
