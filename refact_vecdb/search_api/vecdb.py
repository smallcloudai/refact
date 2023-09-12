import pickle

from pathlib import Path
from typing import List, Optional

import numpy as np

from tqdm import tqdm
from pynndescent import NNDescent

from refact_vecdb.common.profiles import PROFILES as P
from refact_vecdb.common.context import CONTEXT as C


__all__ = ['load_vecdb', 'VecDB']


def load_vecdb(account: str):
    print(f'Loading vecdb for {account}')
    vdb_save = P[account]['workdir'] / 'nn_index.pkl'
    file_chunks_embedding = C.c_models['file_chunks_embedding']
    vecdb = VecDB()

    def fill_vecdb_from_disk():
        embeddings = []
        ids = []
        modified_ts = -1
        record = None
        for record in tqdm(file_chunks_embedding.objects):
            embedding = record.embedding
            embeddings.append(embedding)
            ids.append(record.id)
            modified_ts = max(modified_ts, record.created_ts.timestamp())
        if not record:
            return

        index = NNDescent(np.stack(embeddings, axis=0), low_memory=False)
        index.prepare()

        with vdb_save.open('wb') as f:
            f.write(pickle.dumps({
                'modified_ts': modified_ts,
                'index': index,
                'ids': ids
            }))

        vecdb.from_disk(vdb_save)
    fill_vecdb_from_disk()
    C.vecdb[account] = vecdb


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
