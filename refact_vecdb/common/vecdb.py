import pickle

from typing import List, Optional, Iterable, Dict, Tuple

import numpy as np

from pynndescent import NNDescent

from refact_vecdb.common.context import VDBFiles, CONTEXT as C
from refact_vecdb import VDBSearchAPI

__all__ = ['load_vecdb', 'prepare_vecdb_indexes', 'VecDB']


def retrieve_embeddings(account: str) -> Iterable[Dict]:
    session = C.c_session

    for row in session.execute(
        f"""
        select id, embedding from file_chunks_embedding where account = '{account}';
        """
    ):
        yield row


def prepare_vecdb_indexes(account: str):
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

    with VDBFiles.nn_index.open('wb') as f:
        f.write(pickle.dumps({
            'index': index,
            'ids': ids
        }))
    print(f'vdb_idx prepared for {account}')
    del index
    VDBSearchAPI().update_indexes(account)


def load_vecdb(account: str):
    print(f'Loading vecdb for {account}')
    vdb_save = VDBFiles.nn_index

    if not vdb_save.exists():
        print(f'{vdb_save} not found')
        return

    vecdb = VecDB()
    vecdb.from_disk()
    C.vecdb[account] = vecdb
    print(f'vecdb loaded for {account}')


class VecDB:
    def __init__(self):
        self._index: Optional[NNDescent] = None
        self._ids: Optional[List[str]] = None

    def search(self, embeddings: List, top_k: int = 1) -> Tuple:
        ids, scores = self._index.query(embeddings, k=top_k)
        return [
            [self._ids[i] for i in batch]
            for batch in ids
        ], scores

    def from_disk(self):
        try:
            with VDBFiles.nn_index.open('rb') as f:
                cont = pickle.load(f)
                self._index = cont['index']
                self._ids = cont['ids']
        except FileNotFoundError:
            print(f'{VDBFiles.nn_index} not found')
