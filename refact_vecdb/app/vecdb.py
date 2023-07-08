import pickle

from pathlib import Path
from typing import List, Optional

from pynndescent import NNDescent


class VecDB:
    def __init__(self):
        self._last_date = -1
        self._index: Optional[NNDescent] = None
        self._ids: Optional[List[str]] = None

    def search(self, embeddings: List, top_k: int = 1):
        ids, scores = self._index.query(embeddings, k=top_k)
        return [[self._ids[i] for i in batch] for batch in ids], scores

    @classmethod
    def from_file(cls, filepath: Path):
        db = cls()
        with filepath.open('rb') as f:
            data = pickle.load(f)
            db._index = data['index']
            db._ids = data['ids']
            db._modified_ts = data['modified_ts']
        return db
