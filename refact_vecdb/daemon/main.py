from argparse import ArgumentParser

from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb.daemon.daemon import VDBDaemon

from refact_vecdb.embeds_api.spinup import spinup_models
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

    bootstrap_keyspace("vecdb")
    spinup_models()
    VDBDaemon()()


if __name__ == '__main__':
    main()