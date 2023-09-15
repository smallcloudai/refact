import os
import traceback

from pathlib import Path

import ujson as json

from watchdog.events import FileSystemEventHandler

from refact_vecdb.common.profiles import VDBFiles
from refact_vecdb.common.profiles import PROFILES as P
from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.daemon.params import File2Upload
from refact_vecdb.daemon.crud import \
    get_all_file_names, delete_files_by_name, insert_files, \
    on_model_change_update_embeddings, change_files_active_by_name, set_all_files_active
from refact_vecdb.common.vecdb import prepare_vecdb_indexes
from refact_vecdb import VDBSearchAPI


__all__ = ['WorkDirEventsHandler']


def on_db_set_file_modified(account: str):
    workdir = P[account]['workdir']
    train_set = workdir.joinpath(VDBFiles.train_set)
    test_set = workdir.joinpath(VDBFiles.test_set)

    files_active = []
    files_inactive = []
    for line in [l for l in [*train_set.read_text().splitlines(), *test_set.read_text().splitlines()] if l]:
        line = json.loads(line)
        line = {'path': line['path'], 'to_db': line['to_db']}
        path = P[account]["workdir"].joinpath(line['path'])
        files_active.append(path) if line['to_db'] else files_inactive.append(path)

    files = [*files_active, *files_inactive]
    file_names_db = get_all_file_names(account)
    diff_file_names = set(file_names_db).difference(set(str(p) for p in files))
    if diff_file_names:
        delete_files_by_name(diff_file_names, account)

    if not files:
        return

    insert_files((File2Upload(str(p), p.read_text()) for p in files), account)

    set_all_files_active(account)
    if files_inactive:
        change_files_active_by_name((str(f) for f in files_inactive), account, active=False)


class WorkDirEventsHandler(FileSystemEventHandler):
    def __init__(
            self,
            account: str
    ):
        self._workdir = P[account]['workdir']
        self._account = account
        self._change_provider_file: Path = self._workdir.joinpath(VDBFiles.change_provider)
        self._change_provider_file.unlink(missing_ok=True)

        self._database_set_last_modified = -1
        self._db_set_file = self._workdir.joinpath(VDBFiles.database_set)
        self._db_set_file_modified()

    def _provider_file_changed(self):
        account_data = get_account_data(self._account)
        account_data['provider'] = json.loads(self._change_provider_file.read_text())['provider']
        print(f'change providers file detected; new provider: {account_data["provider"]}')
        update_account_data(account_data)
        on_model_change_update_embeddings(self._account)
        prepare_vecdb_indexes(self._account)
        VDBSearchAPI().update_indexes(self._account, account_data['provider'])
        self._change_provider_file.unlink()

    def _db_set_file_modified(self):
        if os.path.getmtime(self._db_set_file) == self._database_set_last_modified:
            return
        on_db_set_file_modified(self._account)
        prepare_vecdb_indexes(self._account)
        VDBSearchAPI().update_indexes(self._account)
        self._database_set_last_modified = os.path.getmtime(self._db_set_file)

    def on_modified(self, event):
        try:
            if event.src_path.endswith(str(VDBFiles.change_provider)):
                self._provider_file_changed()

            if event.src_path.endswith(str(VDBFiles.database_set)):
                self._db_set_file_modified()

        except Exception:  # noqa
            traceback.print_exc()
