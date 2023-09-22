import traceback
import os

from typing import Dict

import ujson as json

from refact_vecdb.common.context import VDBFiles, upd_status
from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.daemon.crud import on_model_change_update_embeddings, read_and_compare_files
from refact_vecdb.daemon.crud import DBSetChangedException, ConfigChangedException

from refact_vecdb.common.vecdb import prepare_vecdb_indexes
from self_hosting_machinery import env


def mtime_or_0(fn):
    try:
        return os.path.getmtime(fn)
    except OSError:
        return 0


class RaiseIfChanged:
    def __init__(self, fn: str, ExceptionType: type):
        self.fn = fn
        self.mtime = mtime_or_0(fn)
        self.ExceptionType = ExceptionType

    def is_changed(self):
        return self.mtime != mtime_or_0(self.fn)

    def throw_if_changed(self):
        if self.is_changed():
            raise self.ExceptionType()


class ConfigsTracker:
    def __init__(self, account: str):
        self._account = account
        self.keep_track = [
            RaiseIfChanged(env.CONFIG_VECDB, ConfigChangedException),
            RaiseIfChanged(os.path.join(env.DIR_UNPACKED, "database_set.jsonl"), DBSetChangedException)
        ]
        self.update_mtimes()

    def upd_status(self, status: str, message: str) -> None:
        print("status", status, "message", message)
        with open(env.CONFIG_VECDB_STATUS + ".tmp", 'w') as f:
            json.dump({"status": status, "message": message}, f)
        os.rename(env.CONFIG_VECDB_STATUS + ".tmp", env.CONFIG_VECDB_STATUS)

    def upd_stats(self, file_n, total):
        print("file_n", file_n, "total", total)
        with open(env.CONFIG_VECDB_FILE_STATS + ".tmp", 'w') as f:
            json.dump({'file_n': file_n, 'total': total}, f)
        os.rename(env.CONFIG_VECDB_FILE_STATS + ".tmp", env.CONFIG_VECDB_FILE_STATS)

    def sleep_until_configs_change(self) -> None:
        # from inotify_simple import INotify, flags
        # with INotify() as inotify:
        #     inotify.add_watch(env.DIR_UNPACKED, flags.CLOSE_WRITE)
        while 1:
            if self.any_configs_changed():
                break
            time.sleep(1)

    def any_configs_changed(self):
        return any(x.is_changed() for x in self.keep_track)

    def throw_if_configs_changed(self):
        for x in self.keep_track:
            x.throw_if_changed()


def process_all(self, cfg_tracker: ConfigsTracker) -> None:
    try:
        cfg_tracker.upd_status("working", "initializing...")
        account_data = get_account_data(self._account)
        update_account_data(account_data)  # XXX: only to change provider
        cfg_tracker.upd_status("working", "reading files list")
        read_and_compare_files(cfg_tracker, self._account)
        cfg_tracker.upd_status("working", "indexing")
        prepare_vecdb_indexes(cfg_tracker, self._account)
        cfg_tracker.upd_status("idle", "")

    except DBSetChangedException:
        self.upd_status("restarting", "file list changed")

    except ConfigChangedException:
        self.upd_status("restarting", "config changed")

    except Exception as e:  # noqa
        self.upd_status("error", str(e))
        traceback.print_exc()   # prints to stderr, that's what we want

    # GUI is updated every 2 seconds
    # also, if there's an error, we need sleep
    time.sleep(2)


def serve_forever(self):
    cfg_tracker = ConfigsTracker()
    while True:
        # None of that throw any exceptions
        cfg_tracker.sleep_until_configs_change()
        cfg_tracker.update_mtimes(self)
        process_all(self, cfg_tracker)
