from typing import Any, Dict
from pathlib import Path

from dataclasses import dataclass, field

from self_hosting_machinery import env


KEYSPACE = "vecdb"
ACCOUNT = "XXX"


@dataclass
class Context:
    c_session: Any = None
    c_models: Dict[str, Any] = field(default_factory=dict)
    c_setup_data: Dict[str, Any] = field(default_factory=dict)
    vecdb: Dict[str, Any] = field(default_factory=dict)


@dataclass
class VDBFiles:
    workdir = Path(env.DIR_UNPACKED)
    database_set = workdir / "database_set.jsonl"
    file_stats = Path(env.CONFIG_VECDB_FILE_STATS)
    file_stats_tmp = file_stats.with_suffix(".tmp")
    config = Path(env.CONFIG_VECDB)
    status = Path(env.CONFIG_VECDB_STATUS)
    change_provider = Path(env.FLAG_VECDB_CHANGE_PROVIDER)


CONTEXT = Context()
