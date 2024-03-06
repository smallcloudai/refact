import multiprocessing
import os
from typing import Any, Dict

import psutil
import torch
from torch.utils.data import DataLoader
from transformers import AutoTokenizer

from refact_data_pipeline import finetune_datasource
from refact_data_pipeline.datautils import collate_fn, data_parallel_split_and_collate_fn
from self_hosting_machinery.finetune.configuration import supported_models
from refact_utils.scripts.env import TRAIN_FILTERED_FILEPATH, TEST_FILTERED_FILEPATH

__all__ = [
    "setup_encoding",
    "create_train_dataloader",
    "create_test_dataloader",
    "create_finetune_filter_dataloader",
    "get_ds_len_per_epoch",
    "to_cuda",
]


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


def get_ds_len_per_epoch(model_name, cfg_builder):
    encoding = setup_encoding(
        model_name=model_name,
        weights_path=cfg_builder.cfg['model_info']['weight_path'],
        repo_id=cfg_builder.cfg['model_info']['repo_id']
    )
    ds = create_train_dataloader(
        model_name=model_name,
        encoding=encoding,
        num_workers=16,
        batch_size=cfg_builder.cfg['micro_batch_size'],
        ctx_size=cfg_builder.cfg['model_info']['ctx_size'],
        extra_options="quit_on_epoch=1"
    )
    return sum(1 for _ in ds) * int(os.environ.get('WORLD_SIZE', 1))


def create_train_dataloader(
        model_name: str,
        encoding: 'Encoding',
        ctx_size: int,
        batch_size: int,
        num_workers: int,
        extra_options: str = "",
) -> DataLoader:
    world_size = int(os.environ.get('WORLD_SIZE', 1))
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
        jsonl_path=TRAIN_FILTERED_FILEPATH,
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
        batch_size=batch_size * world_size,
        num_workers=8,
        shuffle=False,
        drop_last=True,
        pin_memory=False,
        collate_fn=data_parallel_split_and_collate_fn
    )


def create_test_dataloader(
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
        jsonl_path=TEST_FILTERED_FILEPATH,
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
