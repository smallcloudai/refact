from argparse import ArgumentParser

from refact_vecdb.common.context import CONTEXT as C, KEYSPACE, ACCOUNT
from refact_vecdb.daemon.file_events import run_file_events

from refact_vecdb.common.db_models import bootstrap_keyspace


__all__ = ["main"]


def main():
    parser = ArgumentParser()
    parser.add_argument('--cassandra_host', type=str, default="10.190.99.200")
    parser.add_argument('--cassandra_port', type=int, default=9042)
    args = parser.parse_args()

    hosts = args.cassandra_host
    port = args.cassandra_port

    hosts = hosts if isinstance(hosts, list) else [hosts]
    C.c_setup_data = {
        'hosts': hosts,
        'port': port,
    }

    bootstrap_keyspace(KEYSPACE)
    run_file_events(ACCOUNT)


if __name__ == '__main__':
    main()
