import pickle

from typing import List, Optional

from refact_vecdb.app.context import CONTEXT as C

from pynndescent import NNDescent


class VecDB:
    def __init__(self):
        self._last_date = -1
        self._index: Optional[NNDescent] = None
        self._ids: Optional[List[str]] = None

    def search(self, embeddings: List, top_k: int = 1):
        ids, scores = self._index.query(embeddings, k=top_k)
        return [
            [self._ids[i] for i in batch]
            for batch in ids
        ], scores

    @classmethod
    def from_cassandra(cls):
        db = cls()
        row = list(C.c_session.execute(
            "select vdb_index, vdb_ids from vecdb_data"
        ))[0]
        db._index = pickle.loads(row['vdb_index'])
        db._ids = pickle.loads(row['vdb_ids'])
        return db
