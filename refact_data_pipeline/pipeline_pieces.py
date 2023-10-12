import copy
import datetime
import gzip
import itertools
import os
import random
import traceback
from typing import List, Union, Iterable

import blobfile as bf
import filelock
import ujson
import zstandard
from mpi4py import MPI

from refact_data_pipeline.datadef import DatasetDef, DatasetDumpedDef
from refact_data_pipeline.datadef import DatasetMix
from refact_data_pipeline.datadef import DatasetOpts
from refact_data_pipeline.filters_hdfs import Hdf5Dataset
from refact_data_pipeline.filters_packing import Packer, SinglePacker, DensePacker
from refact_encoding import RefactEncoding

log = print


class JsonlFilesReaderCached:
    def __init__(self,
                 dataopts: DatasetOpts,
                 cloud_path: str,
                 cloud_files: str,
                 datarank: int,
                 cold_restart_key: int,
                 cold_restart_skip: int,
                 ):
        self.cloud_path = cloud_path
        self.cloud_files = cloud_files
        self.datarank = datarank
        self.one_epoch: int = dataopts.get("one_epoch", 0)
        self.cold_restart_key = cold_restart_key
        self.cold_restart_skip = cold_restart_skip

    def __iter__(self):
        record_n = 0
        stats = {}
        short_path = "/".join(self.cloud_path.rstrip("/").split("/")[2:])
        if os.path.exists("/small-cache"):
            cache_dir = os.path.join("/small-cache/", short_path)
        else:
            cache_dir = os.path.join("/tmp/small-cache/", short_path)
        skipped = 0
        for epoch in itertools.count():
            stats["epoch"] = epoch
            stats["datarank"] = self.datarank
            for i, fn in enumerate(self.cloud_files):
                cached_fn = os.path.join(cache_dir, fn)
                stats["task_dir"] = cache_dir
                stats["reading_fn"] = cached_fn
                stats["file_fn"] = fn
                position = epoch * len(self.cloud_files) + i
                stats["file_n"] = position
                stats["file_N"] = len(self.cloud_files)
                stats["file_n_over_N"] = (epoch * len(self.cloud_files) + i) / len(self.cloud_files)
                ymd_hms = datetime.datetime.now().strftime("%Y%m%d %H:%M:%S")
                log(ymd_hms, "epoch %i reading %i/%i %s" % (epoch, i, len(self.cloud_files), cached_fn))
                skipped += 1
                if self.cold_restart_skip > 0 and skipped < self.cold_restart_skip + 2:  # one because it's the same we were reading, and another one for good measure
                    log("skipped %i" % skipped)
                    continue
                stats["restart%02d" % self.cold_restart_key] = position
                os.makedirs(os.path.dirname(cached_fn), exist_ok=True)
                os.umask(0o002)
                with filelock.FileLock(cached_fn + ".lock"):
                    if os.path.exists(cached_fn):
                        pass
                        # This is useful to understand which files are being processed:
                        # log("using cached '%s'" % cached_fn)
                    else:
                        log("downloading '%s' from '%s'" % (cached_fn, self.cloud_path + fn))
                        bf.copy(self.cloud_path + fn, cached_fn + ".tmp")
                        os.rename(cached_fn + ".tmp", cached_fn)
                if fn.endswith(".gz"):
                    it = gzip.open(cached_fn)
                elif fn.endswith(".zst"):
                    def bin2str(buffer_bytes):
                        cctx = zstandard.ZstdDecompressor()
                        with open(cached_fn, "rb") as reader, \
                                cctx.stream_reader(reader) as decompressor:
                            buffer = b""
                            while True:
                                data = decompressor.read(buffer_bytes)
                                if not data:
                                    if buffer:
                                        yield buffer.decode("utf8") + "\n"
                                    break
                                else:
                                    lines = data.split(b"\n")
                                    for idx, line in enumerate(lines[:-1]):
                                        if idx == 0:
                                            line = buffer + line
                                            buffer = b""
                                        yield line.decode("utf8") + "\n"
                                    buffer += lines[-1]

                    it = bin2str(1 << 20)
                else:
                    it = open(cached_fn)
                for line in it:
                    try:
                        d = ujson.loads(line)
                    except ujson.JSONDecodeError:
                        traceback.print_exc()
                        log("line: %r" % line)
                        continue
                    if not isinstance(d, dict):
                        assert isinstance(d, str)
                        d = dict(text=d)
                    stats["record_n"] = record_n
                    record_n += 1
                    d["stats"] = copy.deepcopy(stats)
                    yield d
            if self.one_epoch:
                break


class SplitRanks:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
            commrank: int,
            commsize: int,
    ):
        self.inner_filter = inner_filter
        self.commrank = commrank
        self.commsize = commsize

    def __iter__(self):
        for i, rec in enumerate(self.inner_filter):
            if i % self.commsize == self.commrank:
                yield rec


class Tokenizer:
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts,
                 ):
        self.inner_filter = inner_filter
        self.skip_prompt_len: int = dataopts.get("tkr_skip_long_prompt", 0)
        self.skip_completion_len: int = dataopts.get("tkr_skip_completion_len", 0)
        self.skip_total_len: int = dataopts.get("tkr_skip_total_len", -1)
        if self.skip_total_len == -1:
            self.skip_total_len = 2 ** 31
        self.fatal_skip: bool = dataopts.get("tkr_fatal_skip", 0) == 1
        self.append_eot: bool = dataopts.get("tkr_append_eot", 1) == 1
        self.tkr_stochastic_tokens = dataopts.get("tkr_stochastic_tokens", 0)
        self.tkr_rm_bos_in_completion: int = dataopts.get("tkr_rm_bos_in_completion", 0)
        self.random_seed: int = dataopts.get("seed", 42)
        self.enc = dataopts.encoding
        self.enc.set_random_seed(self.random_seed)
        self.stats = {
            "tkr_skip_prompt_len": 0,
            "tkr_skip_completion_len": 0,
            "tkr_skip_total_len": 0,
            "tkr_success": 0,
        }

    def __iter__(self):
        for ex in self.inner_filter:
            if self.tkr_stochastic_tokens > 0:
                prompt_tokens, _ = self.enc.encode_stochastic(ex["prompt"], [], 0.01 * self.tkr_stochastic_tokens)
                completion_tokens, _ = self.enc.encode_stochastic(ex["completion"], [],
                                                                  0.01 * self.tkr_stochastic_tokens)
            else:
                prompt_tokens = self.enc.encode(ex["prompt"])
                completion_tokens = self.enc.encode(ex["completion"])
                if self.tkr_rm_bos_in_completion:
                    completion_tokens = completion_tokens[1:]
            if self.append_eot:
                completion_tokens.append(self.enc.EOT)
            if self.skip_prompt_len and len(prompt_tokens) > self.skip_prompt_len:
                self.stats["tkr_skip_prompt_len"] += 1
                continue
            if self.skip_completion_len and len(completion_tokens) > self.skip_completion_len:
                self.stats["tkr_skip_completion_len"] += 1
                continue
            if len(prompt_tokens) + len(completion_tokens) > self.skip_total_len:
                self.stats["tkr_skip_total_len"] += 1
                if self.fatal_skip:
                    assert 0, f'too long to tokenize, prompt:\n"{ex["prompt"]}"\ncompletion "{ex["completion"]}"'
                continue
            ex["prompt_tokens"] = prompt_tokens
            ex["completion_tokens"] = completion_tokens
            self.stats["tkr_success"] += 1
            ex["stats"].update(self.stats)
            yield ex


class PromptCompletionToTokensMask:
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts,
                 ):
        self.inner_filter = inner_filter

    def __iter__(self):
        for rec in self.inner_filter:
            ln = len(rec["prompt_tokens"]) + len(rec["completion_tokens"])
            yield {
                "tokens": rec["prompt_tokens"] + rec["completion_tokens"],
                "mask": [0] * len(rec["prompt_tokens"]) + [1] * len(rec["completion_tokens"]),
                "first": [1] + [0] * (ln - 1),
                "diffhlpoint": [0] * ln,  # first position decision of a diff (no such thing for plain text)
                "diffedits": [0] * ln,  # 0 don't learn (1 no edit, 2 edit)
                "stats": rec["stats"],
            }


class Shuffle:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.shuffle_depth: int = dataopts.get("shuffle_depth", 1000)
        self.random_state = random.Random(dataopts.get("seed", 42))

    def __iter__(self):
        buf = []
        for rec in self.inner_filter:
            buf.append(rec)
            if len(buf) >= self.shuffle_depth:
                t = buf.pop(self.random_state.randrange(len(buf)))
                yield t
        while len(buf):
            t = buf.pop(self.random_state.randrange(len(buf)))
            yield t


class Mix:
    def __init__(
            self,
            src: List[Iterable],
            proportions: List[float],
            seed: int,
            shuffle_depth: int = 1000,
    ):
        self.src = src
        self.proportions = proportions if len(proportions) == len(src) else [1 / len(src)] * len(src)
        self.seed = seed
        self.shuffle_depth: int = shuffle_depth
        self.random_state = random.Random(self.seed)
        assert abs(sum(self.proportions) - 1) < 0.0000001

    def __iter__(self):
        buf = []
        iters = [iter(s) for s in self.src]
        accum = [0.0] * len(iters)
        emitted = [0] * len(iters)
        while 1:
            for i in range(len(iters)):
                accum[i] += self.proportions[i]
                # print("%i emitted %i accum %0.2f" % (i, emitted[i], accum[i]))
                if accum[i] > emitted[i]:
                    try:
                        emitted[i] += 1
                        buf.append(next(iters[i]))
                        if len(buf) >= self.shuffle_depth:
                            t = buf.pop(self.random_state.randrange(len(buf)))
                            yield t
                    except StopIteration:
                        assert 0, "It only makes sense to mix infinite datasets"


def build_filter_stack(
        datadef: Union[DatasetDef, DatasetMix],
        dataopts: DatasetOpts,
        enc: RefactEncoding,
        comm: MPI.Comm,
        cold_restart: List[int] = [],
        cold_restart_offset: int = 0,
        skip_assert_flag: bool = False
):
    dataopts.set_encoding(enc)
    if isinstance(datadef, DatasetMix):
        if len(cold_restart) == 0:
            cold_restart = [0] * comm.size * len(datadef.dataset_defs)
        sources = []
        for i, dsdef in enumerate(datadef.dataset_defs):
            cold_restart_offset = i * comm.size
            src = build_filter_stack(
                datadef=dsdef,
                dataopts=dataopts,
                enc=enc,
                comm=comm,
                cold_restart=cold_restart,
                cold_restart_offset=cold_restart_offset,
                skip_assert_flag=True
            )
            sources.append(src)
        return Mix(sources, datadef.proportions, seed=dataopts.get("seed", 42))

    if len(cold_restart) == 0:
        cold_restart = [0] * comm.size
    path = datadef.cloud_path
    files_len = len(datadef.cloud_files)

    if not isinstance(datadef, DatasetDumpedDef):
        if files_len == 1:
            my_files = datadef.cloud_files
        elif files_len % comm.size == 0:
            my_files = [fn for i, fn in enumerate(datadef.cloud_files) if i % comm.size == comm.rank]
        else:
            assert 0, "datadef.cloud_files has %i files, but comm.size is %i" % (files_len, comm.size)
    else:
        my_files = datadef.cloud_files

    log("dataset '%s' has %i files" % (path, len(my_files)))
    assert len(my_files) > 0
    ds = None
    for filt in datadef.to_apply:
        if ds is None and filt == "jsonl":
            ds = JsonlFilesReaderCached(
                dataopts,
                path,
                my_files,
                datarank=comm.rank,
                cold_restart_key=cold_restart_offset + comm.rank,
                cold_restart_skip=cold_restart[cold_restart_offset + comm.rank],
            )
        elif ds is None and filt == 'hdfs':
            ds = Hdf5Dataset(
                dataopts,
                my_files,
                comm=comm,
                cold_restart_skip=cold_restart[cold_restart_offset + comm.rank],
            )
        elif filt == "splitranks":
            ds = SplitRanks(ds, dataopts, commrank=comm.rank, commsize=comm.size)
        elif ds and filt == "tokenize":
            ds = Tokenizer(ds, dataopts)
        elif ds and filt == "tokens+mask":
            ds = PromptCompletionToTokensMask(ds, dataopts)
        elif ds and filt == "pack":
            ds = Packer(ds, dataopts)
        elif ds and filt == "single_pack":
            ds = SinglePacker(ds, dataopts)
        elif ds and filt == "dense_pack":
            ds = DensePacker(ds, dataopts)
        elif ds and filt == "pack16":
            ds = Packer(ds, dataopts, force16=True)
        elif ds and filt == "shuffle":
            ds = Shuffle(ds, dataopts)
        elif ds and not isinstance(filt, str):
            ds = filt(ds, dataopts)
        else:
            assert 0, "cannot apply filter '%s'" % filt
        # log("dataset '%s' filter %s" % (path, ("'%s'" % filt) if isinstance(filt, str) else filt.__name__))
        log("dataset '%s' filter %s" % (path, ds.__class__.__name__))
    if not skip_assert_flag:
        dataopts.assert_all_used()
    return ds
