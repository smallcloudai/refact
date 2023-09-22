from typing import Any, Dict
from pathlib import Path

import ujson as json

from dataclasses import dataclass, field

from self_hosting_machinery import env


@dataclass
class Context:
    c_session: Any = None
    c_models: Dict[str, Any] = field(default_factory=dict)
    c_setup_data: Dict[str, Any] = field(default_factory=dict)
    vecdb: Dict[str, Any] = field(default_factory=dict)


@dataclass
class VDBFiles:
    workdir = Path(env.DIR_UNPACKED)
    nn_index = workdir / 'nn_index.pkl'
    database_set = workdir / "database_set.jsonl"
    file_stats = Path(env.CONFIG_VECDB_FILE_STATS)
    file_stats_tmp = file_stats.with_suffix(".tmp")
    change_provider = Path(env.FLAG_VECDB_CHANGE_PROVIDER)  # GUI only
    config = Path(env.CONFIG_VECDB)
    status = Path(env.CONFIG_VECDB_STATUS)


def upd_file_stats(data: Dict):
    tmp = VDBFiles.file_stats.with_suffix('.tmp')
    with tmp.open('w') as f:
        json.dump(data, f)
    VDBFiles.file_stats.unlink(missing_ok=True)
    tmp.rename(VDBFiles.file_stats)


def upd_status(status: str):
    tmp = VDBFiles.status.with_suffix('.tmp')
    with tmp.open('w') as f:
        json.dump({'status': status}, f)
    VDBFiles.status.unlink(missing_ok=True)
    tmp.rename(VDBFiles.status)


CONTEXT = Context()
