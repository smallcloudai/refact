import os
import traceback

from pathlib import Path
from typing import Dict

import ujson as json

from watchdog.events import FileSystemEventHandler

from refact_vecdb.common.profiles import VDBFiles
from refact_vecdb.daemon.context import CONTEXT as C
from refact_vecdb.daemon.params import File2Upload
from refact_vecdb.daemon.crud import get_all_file_names, delete_files_by_name, insert_files, on_model_change_update_embeddings


__all__ = ['DataBaseSetFileHandler', 'WorkDirEventsHandler']


def create_update_indexes_file(keyspace: str):
    with C.c_sessions[keyspace]['workdir'].joinpath(VDBFiles.update_indexes).open('w') as f:
        f.write(json.dumps({'provider': C.c_sessions[keyspace]['provider']}))


def on_db_set_file_changed(path: Path, keyspace: str):

    def delete_deleted_files() -> None:
        file_names_db = get_all_file_names(keyspace)
        diff_file_names = set(file_names_db).difference(set(str(p) for p in paths_upload))
        if diff_file_names:
            delete_files_by_name(diff_file_names, keyspace)

    text = path.read_text()
    if text:
        paths_upload = [
           C.c_sessions[keyspace]["workdir"].joinpath(p)
           for l in text.splitlines()
           if (p := json.loads(l).get('path'))
        ] or []
    else:
        paths_upload = []

    delete_deleted_files()

    if not paths_upload:
        return

    insert_files((File2Upload(str(p), p.read_text()) for p in paths_upload), keyspace)


class DataBaseSetFileHandler(FileSystemEventHandler):
    def __init__(
            self,
            db_set_file: Path,
            keyspace: str,
    ):
        self._db_set_file = db_set_file
        self._keyspace = keyspace
        self.last_modified = -1
        self.on_modified(None)

    def on_modified(self, event):
        if os.path.getmtime(self._db_set_file) == self.last_modified:
            return
        try:
            on_db_set_file_changed(self._db_set_file, self._keyspace)
            self.last_modified = os.path.getmtime(self._db_set_file)
            create_update_indexes_file(self._keyspace)
        except Exception as e:
            traceback.print_exc()


class WorkDirEventsHandler(FileSystemEventHandler):
    def __init__(
            self,
            workdir: Path,
            keyspace: str
    ):
        self._workdir = workdir
        self._keyspace = keyspace

    def on_created(self, event):
        try:
            if event.src_path.endswith(str(VDBFiles.change_provider)):
                change_provider_file = self._workdir.joinpath(VDBFiles.change_provider)
                C.c_sessions[self._keyspace]['provider'] = json.loads(change_provider_file.read_text())['provider']
                print(f'change providers file detected; new provider: {C.c_sessions[self._keyspace]["provider"]}')
                on_model_change_update_embeddings(self._keyspace)
                create_update_indexes_file(self._keyspace)
                change_provider_file.unlink()
        except Exception as e:
            traceback.print_exc()

