import multiprocessing
from functools import partial
from typing import Any, Dict

import json
import psutil
import torch
import collections
import torch.distributed as dist
from torch.utils.data import DataLoader
from transformers import AutoTokenizer

from refact_data_pipeline import finetune_datasource
from refact_data_pipeline.datautils import collate_fn, data_parallel_split_and_collate_fn
from self_hosting_machinery.finetune.configuration import supported_models


def setup_encoding(
        model_name: str,
        weights_path: str,
        repo_id: str
) -> AutoTokenizer:
    model_config = supported_models.config[model_name]
    assert "tokenizer" in model_config, "Provided tokenizer is no longer supported"
    encoding = AutoTokenizer.from_pretrained(
        repo_id, cache_dir=weights_path,
        trust_remote_code=True
    )
    encoding.decode_utf8 = lambda x, *args, **kwargs: encoding.decode(x)
    encoding.EOT = model_config["tokenizer"]["eot_idx"]
    encoding.DIAMOND = model_config["tokenizer"]["padding_idx"]
    encoding.PREFIX = model_config["tokenizer"]["fim_prefix"]
    encoding.INFILL = model_config["tokenizer"]["fim_middle"]
    encoding.SUFFIX = model_config["tokenizer"]["fim_suffix"]
    encoding.ESCAPE = model_config["tokenizer"]["escape"]
    encoding.BOS = model_config["tokenizer"].get("bos_idx", None)
    return encoding


def get_ds_len_per_epoch(
    train_jsonl_path: str,
    model_name,
    cfg_builder
):
    encoding = setup_encoding(
        model_name=model_name,
        weights_path=cfg_builder.cfg['model_info']['weight_path'],
        repo_id=cfg_builder.cfg['model_info']['repo_id']
    )
    ds = create_train_dataloader(
        jsonl_path=train_jsonl_path,
        model_name=model_name,
        encoding=encoding,
        num_workers=multiprocessing.cpu_count(),
        batch_size=int(cfg_builder.cfg['micro_batch_size'] * dist.get_world_size()),
        ctx_size=cfg_builder.cfg['model_info']['ctx_size'],
        extra_options="quit_on_epoch=1"
    )
    # records like this:
    # {'stats': {'file_num': 1, 'epoch': 0, 'fim_unicode_split': 0, 'fim_unable_to_split': 0, 'fim_out': 1, 'fim_lowlines_skip': 0, 'packed_in': 8, 'packed_out': 3, 'packed_small_dropped': 0, 'last_paddings_perc': 0.021235050036612156},
    # 'tokens': tensor([[    1,   465, 35094,  ...,     4,     4,     4]]),
    # 'mask': tensor([[ True,  True,  True,  ..., False, False, False]]),
    # 'labels': tensor([[  465, 35094,   203,  ...,     4,     4,     4]]),
    # 'input': tensor([[    1,   465, 35094,  ...,     4,     4,     4]])}
    return sum(1 for _ in ds)


def count_file_types(
    jsonl_path: str,
):
    # {"path": "datacollection.tar.bz2/diff_valid/tasks/code_cleanup_unused_variables/orig/factorial.py", "lines": 9, "sloc": 9, "type": "Text", "mime_type": "application/x-python", "language": "Python", "large",
    # : false, "generated": false, "vendored": false, "digits_percent": 0.056074766355140186, "subdir": "datacollection.tar.bz2", "which_set": "train", "to_db": false}
    mime_types_cnt = collections.defaultdict(int)
    with open(jsonl_path) as f:
        for line in f:
            j = json.loads(line)
            mime_types_cnt[j["mime_type"]] += 1
    return mime_types_cnt


def create_train_dataloader(
        jsonl_path,
        model_name: str,
        encoding: 'Encoding',
        ctx_size: int,
        batch_size: int,
        num_workers: int,
        extra_options: str = "",
) -> DataLoader:
    model_config = supported_models.config[model_name]
    ds_name = model_config["train_ds_pipeline"]["ds_name"]
    ds_opts = model_config["train_ds_pipeline"]["ds_opts"].format(
        n_ctx=ctx_size + 1
    )
    if extra_options:
        ds_opts = f"{ds_opts},{extra_options}"

    dataset_cls = getattr(finetune_datasource, ds_name)
    dataset = getattr(finetune_datasource, ds_name).from_a_jsonl(
        cls=dataset_cls,
        jsonl_path=jsonl_path,
        dataset_options=ds_opts,
        encoding=encoding,
    )
    if dataset.files_len == 0:
        raise RuntimeError("No train files provided")

    mem = psutil.virtual_memory()
    if mem.total // 2 ** 30 <= 16:  # saving up a bunch of memory for low specs machines (<= 16Gb ram)
        num_workers = 1

    return DataLoader(
        dataset,
        batch_size=batch_size,
        num_workers=0 if dist.get_world_size() > 1 else num_workers,
        shuffle=False,
        drop_last=True,
        pin_memory=False,
        collate_fn=partial(data_parallel_split_and_collate_fn, global_batch_size=batch_size)
    )


def create_test_dataloader(
    jsonl_path,
    model_name: str,
    encoding: 'Encoding',
    ctx_size: int,
    extra_options: str = "",
) -> DataLoader:
    model_config = supported_models.config[model_name]
    ds_name = model_config["test_ds_pipeline"]["ds_name"]
    ds_opts = model_config["test_ds_pipeline"]["ds_opts"].format(
        n_ctx=ctx_size + 1
    )
    if extra_options:
        ds_opts = f"{ds_opts},{extra_options}"

    dataset_cls = getattr(finetune_datasource, ds_name)
    dataset = getattr(finetune_datasource, ds_name).from_a_jsonl(
        cls=dataset_cls,
        jsonl_path=jsonl_path,
        dataset_options=ds_opts,
        encoding=encoding,
    )
    if dataset.files_len == 0:
        raise RuntimeError("No test files provided")

    return DataLoader(
        dataset,
        batch_size=1,
        num_workers=0,
        shuffle=False,
        drop_last=False,
        pin_memory=False,
        collate_fn=collate_fn
    )


def create_finetune_filter_dataloader(
        file: Dict[str, Any],
        dataset_options: str,
        encoding: str,
) -> DataLoader:
    dataset = finetune_datasource.RefactDataset.from_a_single_file(
        cls=finetune_datasource.RefactPlainCodeDataset,
        file=file,
        dataset_options=dataset_options,
        encoding=encoding
    )
    if dataset.files_len == 0:
        raise RuntimeError("No files for filtering are provided")

    return DataLoader(
        dataset,
        batch_size=1,
        num_workers=0,
        shuffle=False,
        drop_last=False,
        pin_memory=False,
        collate_fn=collate_fn
    )


def to_cuda(batch: Dict[str, Any]) -> Dict[str, Any]:
    return {
        k: (v.cuda() if isinstance(v, torch.Tensor) else v)
        for k, v in batch.items()
    }
