import time

from watchdog.observers import Observer

from refact_vecdb.common.context import VDBFiles
from refact_vecdb.daemon.file_events import WorkDirEventsHandler


class VDBDaemon:
    def __init__(self):
        self._observer = Observer()

    def _spin_up(self):
        for account, data in {'smc': {'workdir': VDBFiles.workdir}}.items():
            workdir = data['workdir']

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

