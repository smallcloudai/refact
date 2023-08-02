import pickle

from pathlib import Path
from typing import List

import numpy as np

from cassandra.auth import PlainTextAuthProvider
from cassandra.cqlengine import connection
from cassandra.cqlengine.management import create_keyspace_simple
from pynndescent import NNDescent
from tqdm import tqdm

from context import CONTEXT as C
from vecdb import VecDB
from db_models import sync_tables, FileChunksEmbedding
from encoder import Encoder

DEFAULT_VECDB_FILEPATH = Path('/tmp/smc.vecdb')


def bootstrap_cassandra_connection(hosts: List[str], port: int, username: str, password: str):
    connection.setup(hosts, 'smc', port=port,
                     auth_provider=PlainTextAuthProvider(username=username, password=password))
    create_keyspace_simple(
        "smc",
        replication_factor=3,
    )
    sync_tables()
    session = connection.get_session()
    session.set_keyspace('smc')
    C.c_session = session


def setup_encoder():
    C.Encoder = Encoder(
        provider='ada',
        instruction='Represent the text:'
    )


def vecdb_from_cassandra():
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
    with DEFAULT_VECDB_FILEPATH.open('wb') as file:
        pickle.dump({
            'index': index,
            'modified_ts': modified_ts,
            'ids': ids
        }, file)

    C.db = VecDB.from_file(DEFAULT_VECDB_FILEPATH)


def load_vecdb():
    vecdb_from_cassandra()
    C.vecdb_update_required = False


def bootstrap(hosts: List[str], port: int, username: str, password: str):
    bootstrap_cassandra_connection(
        hosts=hosts,
        port=port,
        username=username,
        password=password
    )
    setup_encoder()
    load_vecdb()
