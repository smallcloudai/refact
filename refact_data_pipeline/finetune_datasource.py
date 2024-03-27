import os
import random
from typing import Iterable, Dict, Any, List

import jsonlines
import numpy as np
import torch.utils.data

from refact_data_pipeline import DatasetOpts
from refact_data_pipeline import pipeline_pieces as pp
from refact_data_pipeline.datadef import PipelineNode
from refact_data_pipeline.filters_fim_v2 import FIMv2, FIMv2CodeLlama

__all__ = [
    'RefactDataset', 'RefactPlainCodeDataset', 'RefactFIMCodeDataset'
]


class ReadFileByFile(PipelineNode):
    def __init__(
            self,
            basedir: str,
            inner_filter: Iterable[Dict[str, Any]],
            dataopts: DatasetOpts,
    ):
        super().__init__(dataopts)
        assert isinstance(basedir, str), f"basedir must be a string, not {type(basedir).__name__}"
        self.basedir = basedir
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.quit_on_epoch = dataopts.get("quit_on_epoch", 0)
        self.epoch_callback = lambda x: x

    @staticmethod
    def _cut_zip_name(j):
        p = j["path"]
        slash_pos = p.find("/")
        if slash_pos != -1:
            p = p[slash_pos + 1:]
        return p

    def __iter__(self):
        file_num = 0
        epoch = 0
        while 1:
            for j in self.inner_filter:
                code = open(os.path.join(self.basedir, j["path"]), encoding="utf-8").read()
                yield {
                    "path": ReadFileByFile._cut_zip_name(j),
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
            self.epoch_callback(epoch)
            if epoch == self.quit_on_epoch:
                break

    def set_epoch_callback(self, callback):
        self.epoch_callback = callback


class CodeToPrefixCompletion(PipelineNode):
    def __init__(
            self,
            inner_filter: Iterable[Dict[str, Any]],
            dataopts: DatasetOpts,
    ):
        super().__init__(dataopts)
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
            basedir: str,
            files: List[Dict[str, Any]],
            dataset_options: str,
            encoding: 'Encoding',
            change_seed_every_epoch: bool = True
    ):
        self.basedir = basedir
        self._files = files
        self._ds_options = DatasetOpts(dataset_options)
        self._encoding = encoding
        self._change_seed_every_epoch = change_seed_every_epoch
        self._ds_options.set_encoding(self._encoding)

    def get_option(self, name, default):
        return self._ds_options.get(name, default)

    @staticmethod
    def from_a_single_file(
            cls,
            basedir: str,
            file: Dict[str, Any],
            dataset_options: str,
            encoding: 'Encoding'
    ) -> 'RefactDataset':
        return cls(basedir, [file], dataset_options, encoding)

    @staticmethod
    def from_a_jsonl(
            cls,
            jsonl_path: str,
            dataset_options: str,
            encoding: 'Encoding'
    ) -> 'RefactDataset':
        basedir = os.path.dirname(jsonl_path)
        files = list(jsonlines.open(jsonl_path))
        return cls(basedir, files, dataset_options, encoding)

    @property
    def files_len(self) -> int:
        return len(self._files)

    def set_epoch_callback(self, callback):
        assert len(self._pipeline) > 0
        file_reader = self._pipeline[0]
        assert type(file_reader) is ReadFileByFile
        file_reader.set_epoch_callback(callback)

    def set_random_state(self, seed):
        for node in self._pipeline:
            node.set_random_state(seed)

    def _get_files_by_worker(self) -> List[Dict[str, Any]]:
        files = self._files
        random.Random(self._ds_options.get("seed", 42)).shuffle(files)
        worker_info = torch.utils.data.get_worker_info()
        if worker_info is not None:
            assert len(files) > 1, "It doesn't work with 1 file in multiprocessing mode"
            assert len(files) > worker_info.num_workers, "YO have to have more files to process than processes"
            files = np.array_split(files, worker_info.num_workers)[worker_info.id]
        return files

    def _build_pipeline(self, files: List[Dict[str, Any]]):
        raise NotImplementedError()

    def pipeline_nodes(self):
        return self._pipeline

    def __iter__(self):
        self._pipeline = self._build_pipeline(self._get_files_by_worker())
        if self._change_seed_every_epoch:
            self.set_epoch_callback(lambda epoch: self.set_random_state(
                seed=self.get_option("seed", 42) + epoch
            ))
        return iter(self._pipeline[-1])


class RefactPlainCodeDataset(RefactDataset):
    def _build_pipeline(self, files: List[Dict[str, Any]]):
        read_by_file = ReadFileByFile(self.basedir, files, self._ds_options)
        cpc = CodeToPrefixCompletion(read_by_file, self._ds_options)
        tkn = pp.Tokenizer(cpc, self._ds_options)
        pctm = pp.PromptCompletionToTokensMask(tkn, self._ds_options)
        dp = pp.DensePacker(pctm, self._ds_options)
        shf = pp.Shuffle(dp, self._ds_options)
        return [read_by_file, cpc, tkn, pctm, dp, shf]


class RefactFIMCodeDataset(RefactDataset):
    def _build_pipeline(self, files: List[Dict[str, Any]]):
        read_by_file = ReadFileByFile(self.basedir, files, self._ds_options)
        fim = FIMv2(read_by_file, self._ds_options)
        dp = pp.DensePacker(fim, self._ds_options)
        shf = pp.Shuffle(dp, self._ds_options)
        return [read_by_file, fim, dp, shf]


class CodeLLamaFIMDataset(RefactDataset):
    def _build_pipeline(self, files: List[Dict[str, Any]]):
        read_by_file = ReadFileByFile(self.basedir, files, self._ds_options)
        fim = FIMv2CodeLlama(read_by_file, self._ds_options)
        dp = pp.DensePacker(fim, self._ds_options)
        shf = pp.Shuffle(dp, self._ds_options)
        return [read_by_file, fim, dp, shf]
