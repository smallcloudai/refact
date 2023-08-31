from watchdog.observers import Observer

from refact_vecdb.common.profiles import PROFILES, VDBFiles
from refact_vecdb.search_api.file_events import WorkDirEventsHandler


class VDBSearchDaemon:
    def __init__(self):
        self._observer = Observer()

    def spin_up(self):
        for profile in PROFILES:
            profile_name, workdir = profile['name'], profile['workdir']

            self._observer.schedule(
                WorkDirEventsHandler(workdir, profile_name),
                workdir
            )
        self._observer.start()

    def stop(self):
        self._observer.stop()

    def __del__(self):
        print('VDBSearchDaemon is shutting down')
        self._observer.stop()
