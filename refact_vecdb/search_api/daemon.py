from watchdog.observers import Observer

from refact_vecdb.common.profiles import PROFILES
from refact_vecdb.search_api.file_events import WorkDirEventsHandler


class VDBSearchDaemon:
    def __init__(self):
        self._observer = Observer()

    def spin_up(self):
        for account, data in PROFILES.items():
            workdir = data['workdir']

            self._observer.schedule(
                WorkDirEventsHandler(account),
                workdir
            )
        self._observer.start()

    def stop(self):
        self._observer.stop()

    def __del__(self):
        print('VDBSearchDaemon is shutting down')
        self._observer.stop()
