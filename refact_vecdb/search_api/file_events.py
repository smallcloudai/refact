import traceback

import ujson as json

from watchdog.events import FileSystemEventHandler

from refact_vecdb.common.profiles import VDBFiles, PROFILES as P
from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.common.vecdb import load_vecdb


__all__ = ['WorkDirEventsHandler']


class WorkDirEventsHandler(FileSystemEventHandler):
    def __init__(
            self,
            account: str
    ):
        self._account = account
        self._workdir = P[account]['workdir']
        self._update_indexes_file = self._workdir.joinpath(VDBFiles.update_indexes)
        self._update_indexes_file.unlink(missing_ok=True)

    def on_modified(self, event):
        try:
            if event.src_path.endswith(str(VDBFiles.update_indexes)):
                account_data = get_account_data(self._account)
                provider = json.loads(self._update_indexes_file.read_text())['provider']
                account_data['provider'] = provider
                print(f'detected update_indexes for account: {self._account}')
                update_account_data(account_data)
                load_vecdb(self._account)
                self._update_indexes_file.unlink()
        except Exception as e:
            traceback.print_exc()
