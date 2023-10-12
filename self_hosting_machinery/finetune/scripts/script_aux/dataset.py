import os

from torch.utils.data import DataLoader
from transformers import AutoTokenizer

from refact_data_pipeline import finetune_datasource
from refact_data_pipeline.datautils import collate_fn, data_parallel_split_and_collate_fn
from self_hosting_machinery.finetune.configuration import supported_models
from self_hosting_machinery.scripts.env import TRAIN_FILTERED_FILEPATH, TRAIN_UNFILTERED_FILEPATH

__all__ = [
    "create_train_dataloader",
    "create_test_dataloader",
    "get_ds_len_per_epoch",
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
    encoding.encode_stochastic = lambda x, *args, **kwargs: (encoding.encode(x), None)
    encoding.decode_utf8 = lambda x, *args, **kwargs: encoding.decode(x)
    encoding.EOT = model_config["tokenizer"]["eot_idx"]
    encoding.DIAMOND = model_config["tokenizer"]["padding_idx"]
    encoding.PREFIX = model_config["tokenizer"]["fim_prefix"]
    encoding.INFILL = model_config["tokenizer"]["fim_middle"]
    encoding.SUFFIX = model_config["tokenizer"]["fim_suffix"]
    encoding.ESCAPE = model_config["tokenizer"]["escape"]
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
        num_workers=8,
        batch_size=cfg_builder.cfg['model_info']['batch_size'],
        ctx_size=cfg_builder.cfg['model_info']['ctx_size'] + 1
    )
    return sum(1 for _ in ds) * os.environ.get('WORLD_SIZE', 1)


def create_train_dataloader(
        model_name: str,
        encoding: 'Encoding',
        ctx_size: int,
        batch_size: int,
        num_workers: int,
) -> DataLoader:
    world_size = os.environ.get('WORLD_SIZE', 1)
    model_config = supported_models.config[model_name]
    ds_name = model_config["train_ds_pipeline"]["ds_name"]
    ds_opts = model_config["train_ds_pipeline"]["ds_opts"].format(
        n_ctx=ctx_size + 1
    )

    dataset = getattr(finetune_datasource, ds_name)(
        file_path=TRAIN_FILTERED_FILEPATH,
        dataset_options=ds_opts,
        encoding=encoding,
    )
    if dataset.files_len == 0:
        raise RuntimeError("No train files provided")

    return DataLoader(
        dataset,
        batch_size=batch_size * world_size,
        num_workers=num_workers,
        shuffle=False,
        drop_last=True,
        pin_memory=True,
        collate_fn=data_parallel_split_and_collate_fn
    )


def create_test_dataloader(
        model_name: str,
        encoding: 'Encoding',
) -> DataLoader:
    model_config = supported_models.config[model_name]
    ds_name = model_config["test_ds_pipeline"]["pipeline_name"]
    ds_opts = model_config["test_ds_pipeline"]["ds_opts"]

    dataset = getattr(finetune_datasource, ds_name)(
        file_path=TRAIN_UNFILTERED_FILEPATH,
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
        pin_memory=True,
        collate_fn=collate_fn
    )
