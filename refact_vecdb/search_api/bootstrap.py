from pathlib import Path
from typing import List, Union, Any, Dict

from refact_vecdb.common.profiles import PROFILES
from refact_vecdb.search_api.context import CONTEXT as C
from refact_vecdb.common.db_models import bootstrap_keyspace
from refact_vecdb.search_api.vecdb import load_vecdb

__all__ = ['bootstrap', 'setup_keyspace']


def bootstrap(
        hosts: Union[str, List[str]],
        port: int,
) -> None:
    hosts = hosts if isinstance(hosts, list) else [hosts]
    C.c_setup_data = {
        'hosts': hosts,
        'port': port,
    }
    for profile in PROFILES:
        setup_keyspace(profile['name'])


def setup_keyspace(keyspace: str) -> None:
    bootstrap_keyspace(keyspace=keyspace, workdir=Path('/home/user/.refact/tmp/unpacked-files'), context=C)
    load_vecdb(keyspace)
