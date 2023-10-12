import os
import random
from pathlib import Path
from typing import Iterable, Dict, Any, List

import jsonlines
import numpy as np
import torch.utils.data

from refact_data_pipeline import DatasetOpts
from refact_data_pipeline import pipeline_pieces as pp
from refact_data_pipeline.filters_fim_v2 import FIMv2
from self_hosting_machinery import env

__all__ = [
    'RefactPlainCodeDataset', 'RefactFIMCodeDataset'
]


class ReadFileByFile:
    def __init__(
            self,
            inner_filter: Iterable[Dict[str, Any]],
            dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts

    @staticmethod
    def _cut_zip_name(j):
        p = j["path"]
        slash_pos = p.find("/")
        if slash_pos != -1:
            p = p[slash_pos + 1:]
        return p

    def __iter__(self):
        for idx, info in enumerate(self.inner_filter):
            code = open(os.path.join(env.DIR_UNPACKED, info["path"]), encoding="utf-8").read()
            yield {
                "path": ReadFileByFile._cut_zip_name(info),
                "code": code,
                "text": code,
                "size": len(code),
                "stats": {
                    "file_num": idx,
                },
            }


class CodeToPrefixCompletion:
    def __init__(
            self,
            inner_filter: Iterable[Dict[str, Any]],
            dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts

    def __iter__(self):
        for j in self.inner_filter:
            yield {
                "prompt": "FILE %s\n" % j["path"],
                "completion": j["code"],
                "stats": j["stats"],
            }


class RefactDataset(torch.utils.data.IterableDataset):
    def __init__(
            self,
            file_path: str,
            dataset_options: str,
            encoding: 'Encoding'
    ):
        self._file_path = file_path
        self._ds_options = DatasetOpts(dataset_options)
        self._encoding = encoding
        self._ds_options.set_encoding(self._encoding)

    @property
    def files_len(self) -> int:
        files = list(jsonlines.open(Path(env.DIR_UNPACKED) / self._file_path))
        return len(files)

    def _get_files_by_worker(self):
        files = list(jsonlines.open(Path(env.DIR_UNPACKED) / self._file_path))
        random.Random(self._ds_options.get("seed", 42)).shuffle(files)
        worker_info = torch.utils.data.get_worker_info()
        if worker_info is not None:
            files = np.array_split(files, worker_info.num_workers)[worker_info.id]
        return files

    def _build_pipeline(self, files: List[Dict[str, Any]]):
        raise NotImplementedError()

    def __iter__(self):
        return iter(self._build_pipeline(self._get_files_by_worker()))


class RefactPlainCodeDataset(RefactDataset):
    def _build_pipeline(self, files: List[Dict[str, Any]]):
        ds = ReadFileByFile(files, self._ds_options)
        ds = CodeToPrefixCompletion(ds, self._ds_options)
        ds = pp.Tokenizer(ds, self._ds_options)
        ds = pp.PromptCompletionToTokensMask(ds, self._ds_options)
        ds = pp.DensePacker(ds, self._ds_options)
        ds = pp.Shuffle(ds, self._ds_options)
        return ds


class RefactFIMCodeDataset(RefactDataset):
    def _build_pipeline(self, files: List[Dict[str, Any]]):
        ds = ReadFileByFile(files, self._ds_options)
        ds = FIMv2(ds, self._ds_options)
        ds = pp.DensePacker(ds, self._ds_options)
        ds = pp.Shuffle(ds, self._ds_options)
        return ds
