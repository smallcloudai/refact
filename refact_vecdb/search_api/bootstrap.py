from typing import List, Union

from refact_vecdb.common.context import CONTEXT as C, KEYSPACE
from refact_vecdb.common.db_models import bootstrap_keyspace
from refact_vecdb.common.vecdb import load_vecdb


__all__ = ['bootstrap', 'setup_account']


def bootstrap(
        account: str,
        hosts: Union[str, List[str]],
        port: int,
) -> None:
    hosts = hosts if isinstance(hosts, list) else [hosts]
    C.c_setup_data = {
        'hosts': hosts,
        'port': port,
    }
    bootstrap_keyspace(keyspace=KEYSPACE)
    setup_account(account)


def setup_account(account: str) -> None:
    load_vecdb(account)
