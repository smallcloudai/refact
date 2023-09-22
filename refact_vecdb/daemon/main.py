import time

from argparse import ArgumentParser

from refact_vecdb.common.context import CONTEXT as C, upd_status
from refact_vecdb.daemon.daemon import VDBDaemon

from refact_vecdb.common.db_models import bootstrap_keyspace
from refact_vecdb.embeds_api import models
from refact_vecdb import VDBEmbeddingsAPI

__all__ = ["main"]


def test_models_are_running():
    api = VDBEmbeddingsAPI()
    upd_status('OK: wait for models')
    for model in models:
        i = 0
        while True:
            try:
                res = list(api.create({'name': 'test', 'text': 'test'}, provider=model))
                res = res[0]
                assert isinstance(res, dict)
            except Exception:  # noqa
                print(f'Model {model} is not ready yet...')
                upd_status(f'I({i}): model {model} not ready')
                i += 1
                time.sleep(10)
            else:
                print(f'Model {model} is ready')
                upd_status(f'OK: model {model} ready')
                break


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
    test_models_are_running()
    VDBDaemon()()


if __name__ == '__main__':
    main()
