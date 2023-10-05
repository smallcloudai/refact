import os
from pathlib import Path

import jsonlines
import random

from refact_data_pipeline.filters_fim_v2 import FIMv2
from refact_encoding import RefactEncoding
from refact_encoding import hlprint
from refact_data_pipeline import filters_synthetic
from refact_data_pipeline import DatasetOpts
from refact_data_pipeline import pipeline_pieces as pp
from self_hosting_machinery import env

from typing import Union, List, Iterable, Dict, Any, Tuple


def cut_zip_name(j):
    p = j["path"]
    slash_pos = p.find("/")
    if slash_pos != -1:
        p = p[slash_pos + 1:]
    return p


class ReadFileByFile:
    def __init__(
            self,
            inner_filter: Iterable[Dict[str, Any]],
            dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.quit_on_epoch = dataopts.get("quit_on_epoch", 0)

    def __iter__(self):
        file_num = 0
        epoch = 0
        while 1:
            for j in self.inner_filter:
                code = open(os.path.join(env.DIR_UNPACKED, j["path"]), encoding="utf-8").read()
                yield {
                    "path": cut_zip_name(j),
                    "code": code,
                    "text": code,
                    "size": len(code),
                    "stats": {
                        "file_num": file_num,
                        "epoch": epoch,
                    },
                }
                file_num += 1
            epoch += 1
            if epoch == self.quit_on_epoch:
                break


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


from typing import Callable

import torch
import torch.utils.data

from refact_data_pipeline import DatasetOpts


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

    def _get_rank_info(self) -> Tuple[int, int]:
        return os.environ.get('RANK', 0), os.environ.get('WORLD_SIZE', 1)

    def _get_files(self):
        files = list(jsonlines.open(Path(env.DIR_UNPACKED) / self._file_path))
        fixed_seed_random = random.Random(42)
        fixed_seed_random.shuffle(files)
        return files

    @property
    def _pipeline(self):
        raise NotImplementedError()

    def __iter__(self):
        yield self._pipeline()


class RefactPlainDataset(RefactDataset):
    def __init__(
            self,
            dataset_options: str,
            encoding: 'Encoding'
    ):
        super(RefactPlainDataset, self).__init__(dataset_options, encoding)
        self._pipeline = self._build_pipeline()

    def _pipeline(self):
        return self._pipeline

    def _build_pipeline(self):
        rank, size = self._get_rank_info()

        ds = ReadFileByFile(self._get_files(), self._ds_options)
        ds = pp.SplitRanks(ds, self._ds_options, commrank=rank, commsize=size)
        ds = CodeToPrefixCompletion(ds, self._ds_options)
        ds = pp.Tokenizer(ds, self._ds_options)
        ds = pp.PromptCompletionToTokensMask(ds, self._ds_options)
        ds = pp.DensePacker(ds, self._ds_options)
        ds = pp.Shuffle(ds, self._ds_options)
        return ds


def local_plain(fn_set_jsonl: Union[str, List[str]], dataopts):
    rank = 0
    size = 1
    if isinstance(fn_set_jsonl, str):
        js = list(jsonlines.open(os.path.join(env.DIR_UNPACKED, fn_set_jsonl)))
    else:
        js = fn_set_jsonl
    fixed_seed_random = random.Random(43)
    fixed_seed_random.shuffle(js)
    ds = ReadFileByFile(js, dataopts)
    ds = pp.SplitRanks(ds, dataopts, commrank=rank,
                       commsize=size)  # this drops some of the data {"code": ...} at each rank
    ds = CodeToPrefixCompletion(ds, dataopts)
    ds = pp.Tokenizer(ds, dataopts)
    ds = pp.PromptCompletionToTokensMask(ds, dataopts)
    ds = pp.Packer(ds, dataopts, keys=["tokens", "mask", "first"])
    ds = pp.Shuffle(ds, dataopts)
    return ds


def local_fim(fn_set_jsonl, dataopts):
    rank = 0
    size = 1
    if isinstance(fn_set_jsonl, str):
        js = list(jsonlines.open(os.path.join(env.DIR_UNPACKED, fn_set_jsonl)))
    else:
        js = fn_set_jsonl
    fixed_seed_random = random.Random(43)
    fixed_seed_random.shuffle(js)
    ds = ReadFileByFile(js, dataopts)
    ds = pp.SplitRanks(ds, dataopts, commrank=rank,
                       commsize=size)  # this drops some of the data {"code": ...} at each rank
    ds = FIMv2(ds, dataopts)
    ds = pp.DensePacker(ds, dataopts)
    ds = pp.Shuffle(ds, dataopts)
    return iter(ds)
