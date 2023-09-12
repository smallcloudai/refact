import time

from watchdog.observers import Observer

from refact_vecdb.common.profiles import PROFILES, VDBFiles
from refact_vecdb.common.db_models import bootstrap_keyspace
from refact_vecdb.daemon.file_events import DataBaseSetFileHandler, WorkDirEventsHandler


class VDBDaemon:
    def __init__(self):
        self._observer = Observer()

    def _spin_up(self):
        bootstrap_keyspace("vecdb")
        for account, data in PROFILES.items():
            workdir = data['workdir']
            db_set_file = workdir / VDBFiles.database_set
            self._observer.schedule(
                DataBaseSetFileHandler(db_set_file, account),
                db_set_file
            )

            self._observer.schedule(
                WorkDirEventsHandler(account),
                workdir
            )

    def __call__(self):
        self._spin_up()
        self._observer.start()
        while True:
            time.sleep(1)

    def __del__(self):
        self._observer.stop()

