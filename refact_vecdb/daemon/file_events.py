import traceback

from typing import Dict

import ujson as json

from watchdog.events import FileSystemEventHandler, FileSystemEvent

from refact_vecdb.common.context import VDBFiles, upd_status
from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.daemon.crud import on_model_change_update_embeddings, read_and_compare_files
from refact_vecdb.daemon.crud import DBSetChangedException, ConfigChangedException

from refact_vecdb.common.vecdb import prepare_vecdb_indexes


__all__ = ['WorkDirEventsHandler']


class WorkDirEventsHandler(FileSystemEventHandler):
    def __init__(
            self,
            account: str
    ):
        self._account = account
        self.on_modified(FileSystemEvent(str(VDBFiles.database_set)))

    def _on_provider_file_changed(self, account_data: Dict) -> None:
        upd_status('OK: changing model')
        on_model_change_update_embeddings(self._account, account_data['provider'])
        update_account_data(account_data)
        upd_status('OK: preparing indexes')
        prepare_vecdb_indexes(self._account)
        upd_status('OK: indexes prepared')

    def _on_db_set_file_modified(self) -> None:
        read_and_compare_files(self._account)
        upd_status('OK: preparing indexes')
        prepare_vecdb_indexes(self._account)
        upd_status('OK: indexes prepared')

    def on_modified(self, event: FileSystemEvent) -> None:
        try:
            if event.src_path == str(VDBFiles.config):
                data = json.loads(VDBFiles.config.read_text())
                account_data = get_account_data(self._account)
                if data['provider'] != account_data['provider']:
                    account_data['provider'] = data['provider']
                    self._on_provider_file_changed(account_data)
                    VDBFiles.change_provider.unlink()

            if event.src_path == str(VDBFiles.database_set):
                self._on_db_set_file_modified()

        except DBSetChangedException:
            upd_status('I: sources changed')
            self.on_modified(FileSystemEvent(str(VDBFiles.database_set)))

        except ConfigChangedException:
            upd_status(f'I: config changed')
            self.on_modified(FileSystemEvent(str(VDBFiles.config)))

        except Exception as e:  # noqa
            upd_status(f'E: {e}')
            traceback.print_exc()
