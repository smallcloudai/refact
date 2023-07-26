import os
import time
import asyncio
import traceback

from pathlib import Path
from typing import List, Union, Iterable

import ujson as json

from self_hosting_machinery import env

from refact_vecdb.common.context import VDBFiles
from refact_vecdb.common.crud import get_account_data, update_account_data
from refact_vecdb.daemon.crud import on_model_change_update_embeddings, read_and_compare_files
from refact_vecdb.common.vecdb import prepare_vecdb_indexes

from refact_vecdb import VDBEmbeddingsAPI


__all__ = ['run_file_events']


class DBSetChangedException(Exception):
    pass


class ConfigChangedException(Exception):
    pass


def mtime_or_0(file_name: Union[str, Path]) -> float:
    try:
        return os.path.getmtime(str(file_name))
    except Exception:  # noqa
        return 0


class RaiseIfChanged:
    def __init__(self, file_name: str, ExceptionType: type):
        self.file_name = file_name
        self.ExceptionType = ExceptionType
        self.update_mtime()

    def is_changed(self) -> bool:
        return self.mtime != mtime_or_0(self.file_name)

    def throw_if_changed(self) -> None:
        if self.is_changed():
            raise self.ExceptionType()

    def update_mtime(self) -> None:
        self.mtime = mtime_or_0(self.file_name)  # noqa


class ConfigsTracker:
    def __init__(self, account: str):
        self.account = account
        self._db_set = os.path.join(env.DIR_UNPACKED, "database_set.jsonl")
        self.keep_track = [
            RaiseIfChanged(env.CONFIG_VECDB, ConfigChangedException),
            RaiseIfChanged(self._db_set, DBSetChangedException)
        ]
        self.update_mtimes()

    def sleep_until_configs_change(self) -> None:
        # from inotify_simple import INotify, flags
        # with INotify() as inotify:
        #     inotify.add_watch(env.DIR_UNPACKED, flags.CLOSE_WRITE)
        while 1:
            if self.files_changed():
                break
            time.sleep(1)

    def update_mtimes(self) -> None:
        [x.update_mtime() for x in self.keep_track]

    def upd_status(self, status: str, detailed: str = '') -> None:
        print(f"status: {status}; detailed: {detailed}")
        with open(env.CONFIG_VECDB_STATUS + ".tmp", 'w') as f:
            json.dump({"status": status, "detailed": detailed}, f)
        os.rename(env.CONFIG_VECDB_STATUS + ".tmp", env.CONFIG_VECDB_STATUS)

    def upd_stats(self, file_n: int, total: int) -> None:
        print("file_n", file_n, "total", total)
        with open(env.CONFIG_VECDB_FILE_STATS + ".tmp", 'w') as f:
            json.dump({'file_n': file_n, 'total': total}, f)
        os.rename(env.CONFIG_VECDB_FILE_STATS + ".tmp", env.CONFIG_VECDB_FILE_STATS)

    def files_changed(self) -> List[str]:
        return [x.file_name for x in self.keep_track if x.is_changed()]

    def throw_if_changed(self) -> None:
        for x in self.keep_track:
            x.throw_if_changed()


async def wait_for_models(
    cfg_tracker: ConfigsTracker,
    models: Union[str, Iterable[str]]
) -> None:
    api = VDBEmbeddingsAPI()
    models = [models] if isinstance(models, str) else models
    while True:
        i = 0
        for model in models:
            try:
                res = list(api.create({'name': 'test', 'text': 'test'}, provider=model))[0]
                assert isinstance(res, dict)
                print(f'Model {model} is ready')
                cfg_tracker.upd_status(f'OK: model {model} ready')
                return
            except Exception:  # noqa
                print(f'Model {model} is not ready yet...')
                cfg_tracker.upd_status(f'I({i}): model {model} not ready')
                i += 1
                await asyncio.sleep(10)


async def process_all(cfg_tracker: ConfigsTracker) -> None:
    async def on_db_set_mod():
        pass

    async def on_cfg_mod():
        async def on_provider_changed():
            cfg_tracker.upd_status("model change")
            await wait_for_models(cfg_tracker, provider)
            on_model_change_update_embeddings(cfg_tracker, provider)
            account_data.update(dict(provider=provider))
            update_account_data(account_data)
            VDBFiles.change_provider.unlink(missing_ok=True)

        data = json.loads(Path(file_name).read_text())
        if (provider := data['provider']) != account_data['provider']:
            await on_provider_changed()

    try:
        if not (files_changed := cfg_tracker.files_changed()):
            raise Exception("no files changed")
        cfg_tracker.update_mtimes()

        cfg_tracker.upd_status("init")
        account_data = get_account_data(cfg_tracker.account)

        for file_name in files_changed:
            if file_name.endswith('database_set.jsonl'):
                await on_db_set_mod()
            elif file_name == env.CONFIG_VECDB:
                await on_cfg_mod()

        cfg_tracker.upd_status("updating files")
        read_and_compare_files(cfg_tracker, account_data['provider'])
        cfg_tracker.upd_status("indexing")
        prepare_vecdb_indexes(cfg_tracker.account)
        cfg_tracker.upd_stats(1, 1)
        cfg_tracker.upd_status("idle")

    except DBSetChangedException:
        cfg_tracker.upd_status("restarting", "file list changed")

    except ConfigChangedException:
        cfg_tracker.upd_status("restarting", "config changed")

    except Exception as e:  # noqa
        cfg_tracker.upd_status("error", str(e))
        traceback.print_exc()  # prints to stderr

    await asyncio.sleep(2)  # GUI is updated every 2 seconds


async def serve_forever(account) -> None:
    async def wait_for_model() -> None:
        while True:
            try:
                account_data = get_account_data(cfg_tracker.account)
                provider = account_data['provider']
                assert provider
            except Exception as e:  # noqa
                cfg_tracker.upd_status('no provider')
                print(f"Failed to fetch provider: {e}")
            else:
                await wait_for_models(cfg_tracker, provider)
                return
            await asyncio.sleep(1)

    cfg_tracker = ConfigsTracker(account)
    await wait_for_model()
    while True:
        # None of that throw any exceptions
        cfg_tracker.sleep_until_configs_change()
        await process_all(cfg_tracker)


async def job(accounts: List[str]):
    tasks = []
    for account in accounts:
        tasks.append(asyncio.create_task(serve_forever(account)))
    await asyncio.gather(*tasks)


def run_file_events(account: str):
    asyncio.run(job([account]))
