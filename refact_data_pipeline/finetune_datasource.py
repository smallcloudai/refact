import os
import jsonlines
import random

from refact_data_pipeline.filters_fim_v2 import FIMv2
from refact_encoding import RefactEncoding
from refact_encoding import hlprint
from refact_data_pipeline import filters_synthetic
from refact_data_pipeline import DatasetOpts
from refact_data_pipeline import pipeline_pieces as pp
from self_hosting_machinery import env

from typing import Union, List


def cut_zip_name(j):
    p = j["path"]
    slash_pos = p.find("/")
    if slash_pos != -1:
        p = p[slash_pos+1:]
    return p


class ReadFileByFile:
    def __init__(
        self,
        js,
        dataopts: DatasetOpts,
    ):
        self.js = js
        self.dataopts = dataopts
        self.quit_on_epoch = dataopts.get("quit_on_epoch", 0)

    def __iter__(self):
        file_num = 0
        epoch = 0
        while 1:
            for j in self.js:
                # print("READING", j["path"])
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
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts,
                 ):
        self.inner_filter = inner_filter

    def __iter__(self):
        for j in self.inner_filter:
            yield {
                "prompt": "FILE %s\n" % j["path"],
                "completion": j["code"],
                "stats": j["stats"],
            }


def local_infill(fn_set_jsonl, dataopts):
    rank = 0
    size = 1
    js = list(jsonlines.open(os.path.join(env.DIR_UNPACKED, fn_set_jsonl)))
    fixed_seed_random = random.Random(42)
    fixed_seed_random.shuffle(js)
    ds = ReadFileByFile(js, dataopts)
    ds = pp.SplitRanks(ds, dataopts, commrank=rank, commsize=size)
    ds = filters_synthetic.InfillDiff(ds, dataopts)
    ds = pp.Packer(ds, dataopts, keys=["tokens", "mask", "first"], force16=True, force_pack_complete=True)
    ds = pp.Shuffle(ds, dataopts)
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
    ds = pp.SplitRanks(ds, dataopts, commrank=rank, commsize=size)   # this drops some of the data {"code": ...} at each rank
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
    ds = pp.SplitRanks(ds, dataopts, commrank=rank, commsize=size)   # this drops some of the data {"code": ...} at each rank
    ds = FIMv2(ds, dataopts)
    ds = pp.DensePacker(ds, dataopts)
    ds = pp.Shuffle(ds, dataopts)
    return iter(ds)


def local_mix_plain_infill(fn_set_jsonl, dataopts):
    return pp.Mix([
        local_plain(fn_set_jsonl, dataopts),
        local_infill(fn_set_jsonl, dataopts),
    ], (0.75, 0.25))


def local_sequence_plain_infill(fn_set_jsonl, dataopts):
    ds1 = local_plain(fn_set_jsonl, dataopts)
    ds2 = local_infill(fn_set_jsonl, dataopts)
    def _iter():
        for ex1 in ds1:
            yield ex1
        for ex2 in ds2:
            yield ex2
    return _iter()


def print_data_feed(is_test_set):
    enc = RefactEncoding("openai_programming_v2")
    if is_test_set:
        dataopts = DatasetOpts("n_ctx=2049,quit_on_epoch=1,seed=1337")
        dataopts.set_encoding(enc)
        ds = local_sequence_plain_infill("test_set.jsonl", dataopts)
    else:
        dataopts = DatasetOpts("n_ctx=2049,seed=1337")
        dataopts.set_encoding(enc)
        ds = local_mix_plain_infill("train_set.jsonl", dataopts)
    cnt = 0
    for ex in ds:
        print(hlprint(enc, ex["tokens"], ex["mask"]))
        cnt += 1
        if cnt == 10:
            break


if __name__ == '__main__':
    print_data_feed(is_test_set=False)
