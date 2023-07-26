import pickle
import uuid

from typing import List, Optional, Iterable, Dict, Tuple

import numpy as np

from pynndescent import NNDescent

from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb import VDBSearchAPI


__all__ = ['load_vecdb', 'prepare_vecdb_indexes', 'VecDB']


def retrieve_embeddings(account: str) -> Iterable[Dict]:
    session = C.c_session

    yield from session.execute(
        f"""
        select id, embedding from file_chunks_embedding where account = '{account}';
        """
    )


def prepare_vecdb_indexes(account: str):
    session = C.c_session
    print(f'preparing vdb_idx for {account}')

    embeddings = []
    ids = []
    for row in retrieve_embeddings(account):
        embeddings.append(row['embedding'])
        ids.append(row['id'])

    print(f'{len(embeddings)} embeddings')
    if not embeddings:
        return
    index = NNDescent(np.stack(embeddings, axis=0), low_memory=False)
    index.prepare()

    # delete old index
    for r in session.execute(
        session.prepare('SELECT id FROM nn_index WHERE account = ?'),
        [account]
    ):
        id_ = r['id']
        session.execute(
            session.prepare('DELETE FROM nn_index WHERE id = ? AND account = ?'),
            [id_, account]
        )

    session.execute(
        session.prepare('INSERT INTO nn_index (id, account, nn_index, nn_ids) VALUES (?, ?, ?, ?)'),
        [str(uuid.uuid4()), account, pickle.dumps(index), pickle.dumps(ids)]
    )
    print(f'vdb_idx prepared for {account}')
    del index
    VDBSearchAPI().update_indexes(account)


def load_vecdb(account: str):
    print(f'Loading vecdb for {account}')
    vecdb = VecDB()
    vecdb.from_db(account)
    C.vecdb[account] = vecdb
    print(f'vecdb loaded for {account}')


class VecDB:
    def __init__(self):
        self._index: Optional[NNDescent] = None
        self._ids: Optional[List[str]] = None

    def search(self, embeddings: List, top_k: int = 1) -> Tuple:
        ids, scores = self._index.query(embeddings, k=top_k)
        try:
            return [
                [self._ids[i] for i in batch]
                for batch in ids
            ], scores
        except KeyError:
            raise

    def from_db(self, account: str):
        session = C.c_session

        if not (row := session.execute(
            f"""
            select nn_index, nn_ids from nn_index where account = '{account}';
            """
        ).one()):
            return

        self._index, self._ids = pickle.loads(row['nn_index']), pickle.loads(row['nn_ids'])
