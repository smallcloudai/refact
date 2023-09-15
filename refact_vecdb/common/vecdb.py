import pickle

from pathlib import Path
from typing import List, Optional

import numpy as np

from pynndescent import NNDescent

from refact_vecdb.common.profiles import PROFILES as P
from refact_vecdb.common.context import CONTEXT as C
from refact_vecdb.daemon.crud import get_all_active_embeddings


__all__ = ['load_vecdb', 'prepare_vecdb_indexes', 'VecDB']


def prepare_vecdb_indexes(account: str):
    print(f'preparing vdb_idx for {account}')
    vdb_save = P[account]['workdir'] / 'nn_index.pkl'

    embeddings = []
    ids = []

    for row in get_all_active_embeddings(account):
        embeddings.append(row['embedding'])
        ids.append(row['id'])

    print(f'{len(embeddings)} embeddings')
    index = NNDescent(np.stack(embeddings, axis=0), low_memory=False)
    index.prepare()

    with vdb_save.open('wb') as f:
        f.write(pickle.dumps({
            'index': index,
            'ids': ids
        }))
    print(f'vdb_idx prepared for {account}')


def load_vecdb(account: str):
    print(f'Loading vecdb for {account}')
    vdb_save = P[account]['workdir'] / 'nn_index.pkl'
    if not vdb_save.exists():
        print(f'{vdb_save} not found')
        return
    vecdb = VecDB()
    vecdb.from_disk(vdb_save)
    C.vecdb[account] = vecdb
    print(f'vecdb loaded for {account}')


class VecDB:
    def __init__(self):
        self._index: Optional[NNDescent] = None
        self._ids: Optional[List[str]] = None

    def search(self, embeddings: List, top_k: int = 1):
        ids, scores = self._index.query(embeddings, k=top_k)
        return [
            [self._ids[i] for i in batch]
            for batch in ids
        ], scores

    def from_disk(self, file: Path):
        with file.open('rb') as f:
            cont = pickle.load(f)
            self._index = cont['index']
            self._ids = cont['ids']
