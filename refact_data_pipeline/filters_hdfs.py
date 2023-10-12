from pathlib import Path
from typing import Tuple, Optional, Any, List, Dict

import mpi4py.MPI as mpi
import numpy as np
import tables as tb

from refact_data_pipeline import DatasetOpts


def _try_open(path: Path) -> Optional[Any]:
    try:
        return tb.open_file(str(path), mode='r')
    except Exception as e:
        print(f'Cannot open the file {path}: {e}')
        return None


class Hdf5Dataset:
    """
    A class that maps HDF5 files to flat array of data

    Parameters
    ----------
    comm : Optional[mpi4py.MPI.Comm]
        The MPI communicator.
    """

    def __init__(
            self,
            dataopts: DatasetOpts,
            files: List[Path],
            comm: Optional[mpi.Comm] = None,
            cold_restart_skip: Optional[int] = None
    ):
        files = [_try_open(p) for p in files]
        files = [f for f in files if f is not None]
        assert len(files) > 0
        self.files = files
        self.tables = [file.root.data for file in self.files]
        self.keys = dataopts.get("keys", "tokens;mask").split(';')
        self.seed = dataopts.get("seed", 42)
        self.comm = comm
        self.cold_restart_skip = cold_restart_skip
        self.tables_lengths = [len(t) for t in self.tables]
        self.tables_lengths_cumsum = np.cumsum(self.tables_lengths)
        self.overall_length = self.tables_lengths_cumsum[-1]
        self.index = self.__reshuffle()
        self.tables_iter = None

    def __del__(self):
        for file in self.files:
            file.close()

    def __reshuffle(self) -> np.ndarray:
        rng = np.random.default_rng(self.seed)
        index = rng.choice(self.overall_length, self.overall_length, replace=False)

        if self.comm is not None:
            rank_len = len(index) // self.comm.size
            index = index[:rank_len * self.comm.size]
            index = index[self.comm.rank * rank_len:(self.comm.rank + 1) * rank_len]

        return index

    def reshuffle(self):
        assert self.manual_seed is None, "`reshuffle` with the manual seed leads to do nothing, it may be a bug"
        self.index = self.__reshuffle()
        assert self.tables_iter is None, "`reshuffle` cannot be called while iterating"

    def __len__(self):
        return len(self.index)

    def __next__(self) -> Dict[str, Any]:
        assert self.tables_iter is not None, "`__next__` called before `__iter__`"
        iter_n, idx = next(self.tables_iter)
        data = dict(zip(self.keys, self[idx]))
        data['stats'] = dict(record_n=int(iter_n), restart=int(iter_n))
        return data

    def __iter__(self) -> 'Hdf5Dataset':
        self.tables_iter = iter(enumerate(self.index))
        if self.cold_restart_skip is not None:
            for _ in range(self.cold_restart_skip):
                next(self.tables_iter)
            self.cold_restart_skip = None
        return self

    def __getitem__(self, idx: int) -> Tuple[Any, ...]:
        table_idx, table_cumsum = next(
            ((i, t) for i, t in enumerate(self.tables_lengths_cumsum) if idx < t)
        )
        row_idx = idx - (table_cumsum - self.tables_lengths[table_idx]) - 1
        row = self.tables[table_idx][row_idx]
        return tuple(row[k].tolist() for k in self.keys)
