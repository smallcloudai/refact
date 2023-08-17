import uuid
import pickle

from pathlib import Path
from typing import List, Union

import numpy as np

from cassandra.auth import PlainTextAuthProvider
from cassandra.cqlengine import connection
from cassandra.cqlengine.management import create_keyspace_simple
from pynndescent import NNDescent
from tqdm import tqdm

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.vecdb import VecDB
from refact_vecdb.app.db_models import sync_tables, FileChunksEmbedding, VecdbData
from refact_vecdb.app.encoder import VecDBEncoder

DEFAULT_VECDB_FILEPATH = Path('/tmp/smc.vecdb')


def bootstrap_cassandra_connection(hosts: List[str], port: int, username: str, password: str):
    connection.setup(
        hosts, 'smc',
        port=port,
        auth_provider=PlainTextAuthProvider(username=username, password=password)
    )
    create_keyspace_simple("smc", replication_factor=1)
    sync_tables()
    session = connection.get_session()
    C.c_session = session
    C.c_session.set_keyspace('smc')


def setup_encoder():
    C.encoder = VecDBEncoder()


def load_vecdb():
    def fill_vecdb_from_cassandra():
        embeddings = []
        ids = []
        modified_ts = -1
        record = None
        for record in tqdm(FileChunksEmbedding.objects):
            embedding = record.embedding
            embeddings.append(embedding)
            ids.append(record.id)
            modified_ts = max(modified_ts, record.created_ts.timestamp())
        if not record:
            return

        index = NNDescent(np.stack(embeddings, axis=0), low_memory=False)
        index.prepare()

        C.c_session.execute('TRUNCATE vecdb_data;')
        VecdbData.create(**{
            'id': str(uuid.uuid4()),
            'vdb_index': pickle.dumps(index),
            'vdb_ids': pickle.dumps(ids)
        })
        C.vecdb = VecDB.from_cassandra()

    fill_vecdb_from_cassandra()
    C.vecdb_update_required = False


def bootstrap(provider, hosts: Union[str, List[str]], port: int, username: str, password: str):
    C.provider = provider
    hosts = hosts if isinstance(hosts, list) else [hosts]
    bootstrap_cassandra_connection(
        hosts=hosts,
        port=port,
        username=username,
        password=password
    )
    setup_encoder()
    load_vecdb()
