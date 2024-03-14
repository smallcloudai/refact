import sys
import click
import copy
import json
import logging
import multiprocessing
import os
import signal
import time
from pathlib import Path
from typing import Dict, Any, Iterable, Tuple

import torch as th
import torch.distributed as dist

from refact_utils.scripts import env
from refact_utils.finetune.utils import get_finetune_config
from self_hosting_machinery.finetune.configuration.finetune_config import base_config, ConfigBuilder
from self_hosting_machinery.finetune.scripts.auxiliary.dataset import (
    create_train_dataloader, create_test_dataloader, get_ds_len_per_epoch, to_cuda, count_file_types
)
from self_hosting_machinery.finetune.scripts.auxiliary.early_stopper import EarlyStopper
from self_hosting_machinery.finetune.scripts.auxiliary.finetune_status_tracker import FinetuneStatusTracker
from self_hosting_machinery.finetune.scripts.auxiliary.model import ModelContext
from self_hosting_machinery.finetune.utils import traces
from refact_utils.finetune.utils import default_finetune_model

from refact_utils.scripts.env import TRAIN_FILTERED_FILEPATH, TEST_FILTERED_FILEPATH


def _log_everywhere(message):
    if dist.get_rank() != 0:
        return
    logging.info(message)
    traces.log(message)


from refact_utils.finetune.train_defaults import finetune_train_defaults

@click.command()
@click.option('--project', default='')
@click.option('--limit_time_seconds', default=finetune_train_defaults['limit_time_seconds'])
@click.option('--lr', default=finetune_train_defaults['lr'])
@click.option('--batch_size', default=finetune_train_defaults['batch_size'])
@click.option('--warmup_num_steps', default=finetune_train_defaults['warmup_num_steps'])
@click.option('--weight_decay', default=finetune_train_defaults['weight_decay'])
@click.option('--use_heuristics', default=finetune_train_defaults['use_heuristics'])
@click.option('--train_steps', default=finetune_train_defaults['train_steps'])
@click.option('--lr_decay_steps', default=finetune_train_defaults['lr_decay_steps'])
@click.option('--lora_r', default=finetune_train_defaults['lora_r'])
@click.option('--lora_alpha', default=finetune_train_defaults['lora_alpha'])
@click.option('--lora_dropout', default=finetune_train_defaults['lora_dropout'])
@click.option('--trainable_embeddings', default=finetune_train_defaults['trainable_embeddings'])
@click.option('--low_gpu_mem_mode', default=finetune_train_defaults['low_gpu_mem_mode'])
@click.option('--model_name', default=default_finetune_model)
def _build_finetune_config_by_heuristics(project, **kwargs) -> Dict[str, Any]:
    from known_models_db.refact_known_models import models_mini_db
    models_db: Dict[str, Any] = copy.deepcopy(models_mini_db)
    with open(env.CONFIG_FINETUNE_FILTER_STAT, 'r') as f:
        initial_loss = json.load(f)["avg_loss"]
    user_cfg = copy.deepcopy(finetune_train_defaults)
    user_cfg_nondefault = {}
    for k, v in kwargs.items():
        # traces.log("Command line parameter: %s = %s" % (k, v))
        user_cfg[k] = v
        if finetune_train_defaults.get(k, 0) != v:
            user_cfg_nondefault[k] = v

    cfg_builder = ConfigBuilder(base_config(kwargs['model_name'], models_db))
    if user_cfg['use_heuristics']:
        _log_everywhere("Retrieving dataset length per epoch, it may take a while...")
        ds_len = get_ds_len_per_epoch(env.TRAIN_FILTERED_FILEPATH, kwargs['model_name'], cfg_builder)
        traces.log(f"Dataset length per epoch = {ds_len}")
        (cfg_builder
         .set_batch_size(cfg_builder.cfg['train_batch_size'])
         .set_lora_quality_by_heuristics(ds_len=ds_len, initial_loss=initial_loss)
         .set_schedule_by_heuristics(ds_len=ds_len)
         .set_low_gpu_mem_mode_by_heuristics())
    else:
        _log_everywhere("Using finetune setup parameters")
        (cfg_builder
         .set_train_steps(user_cfg['train_steps'])
         .set_lr_decay_steps(user_cfg['lr_decay_steps'])
         .set_lora_r(user_cfg['lora_r'])
         .set_lora_alpha(user_cfg['lora_alpha'])
         .set_lora_dropout(user_cfg['lora_dropout'])
         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
        (cfg_builder
         .set_lr(user_cfg['lr'])
         .set_batch_size(user_cfg['batch_size'])
         .set_warmup_steps(user_cfg['warmup_num_steps'])
         .set_limit_time_seconds(user_cfg['limit_time_seconds'])
         .set_weight_decay(user_cfg['weight_decay']))

    if dist.get_rank() == 0:
        filetypes_train = count_file_types(env.TRAIN_FILTERED_FILEPATH)
        filetypes_test = count_file_types(env.TEST_FILTERED_FILEPATH)
        traces.log(f'Train file types:')
        for k, v in filetypes_train.items():
            traces.log(f'    {v} {k}')
        traces.log(f'')
        traces.log(f'Test file types:')
        for k, v in filetypes_test.items():
            traces.log(f'    {v} {k}')
        traces.log(f'')
        with open(os.path.join(traces.context().path, "source_files.json"), "w") as f:
            json.dump({
                "project": project,
                "train": filetypes_train,
                "test": filetypes_test,
            }, f, indent=4)

    if dist.get_rank() == 0:
        for k, v in user_cfg_nondefault.items():
            traces.log(f'Non-default parameter: {k:>20} {v}')
        with open(os.path.join(traces.context().path, "parameters_nondefault.json"), "w") as f:
            json.dump(user_cfg_nondefault, f, indent=4)
        traces.log(f'Freeze exceptions: {cfg_builder.cfg["model_info"]["freeze_exceptions"]}')
        for k, v in cfg_builder.cfg["model_info"]["lora"].items():
            traces.log(f'Lora config: {k:>20} {v}')
        with open(os.path.join(traces.context().path, "config.json"), "w") as f:
            json.dump(cfg_builder.cfg, f, indent=4)
        traces.log(f'Low memory mode: {user_cfg["low_gpu_mem_mode"]}')

    assert cfg_builder.cfg['train_iters'] % cfg_builder.cfg['test_every'] == 0
    assert cfg_builder.cfg['save_every'] % cfg_builder.cfg['test_every'] == 0

    return cfg_builder.cfg


def _train_iteration(
        data: Dict[str, Any],
        iter_n: int,
        model_context: ModelContext,
        finetune_cfg: Dict[str, Any],
        status_tracker: FinetuneStatusTracker,
) -> Tuple[float, int]:
    world_size = dist.get_world_size()
    zero_rank = dist.get_rank() == 0

    if zero_rank and finetune_cfg['debug']:
        data_path = Path(traces.context().path) / ('debug_data/iter%04d' % iter_n)
        data_path.mkdir(exist_ok=True, parents=True)

    losses, tokens_n = [], 0
    for b0 in range(0, finetune_cfg["train_batch_size"] // world_size, finetune_cfg["micro_batch_size"]):
        input = data['input'][b0:b0 + finetune_cfg["micro_batch_size"]].contiguous()
        logits = model_context.forward(input=input)
        loss = model_context.loss(
            logits=logits,
            labels=data['labels'][b0:b0 + finetune_cfg["micro_batch_size"]].contiguous(),
            mask=data['mask'][b0:b0 + finetune_cfg["micro_batch_size"]].contiguous(),
        )
        model_context.backward(loss)
        model_context.step()
        tokens_n += (input.shape[0] * input.shape[1]) * world_size
        losses.append(loss.item())
        status_tracker.update_status("working")

        if zero_rank and finetune_cfg['debug']:
            with open(data_path / ('%d_%0.3f.txt' % (b0, loss.item())), 'w') as f:
                f.write(model_context.encoding.decode(input[0].cpu().numpy()))

    return sum(losses) / len(losses), tokens_n


def _test_iteration(
        test_ds: Iterable[Dict[str, Any]],
        iter_n: int,
        model_context: ModelContext,
        finetune_cfg: Dict[str, Any],
) -> float:
    if finetune_cfg["test_every"] > 0 and iter_n % finetune_cfg["test_every"] == 0:
        model_context.eval()
        with th.inference_mode():
            losses = []
            for batch in map(to_cuda, test_ds):
                logits = model_context.forward(input=batch['input'])
                loss = model_context.loss(
                    logits=logits,
                    labels=batch['labels'],
                    mask=batch['mask'],
                )
                losses.append(loss.item())

        model_context.train()
        return sum(losses) / len(losses)


def loop(
    train_jsonl_path: str,
    test_jsonl_path: str,
    finetune_cfg: Dict[str, Any],
    model_context: ModelContext,
    status_tracker: FinetuneStatusTracker
):
    def _save_checkpoint(iter_n: int, loss: float, force: bool = False):
        if not zero_rank:
            return
        if force or (iter_n != 0 and iter_n % finetune_cfg['save_every'] == 0):
            tag = f"iter{iter_n:04d}-testloss{loss:.3f}"
            traces.log(f"Saving checkpoint {tag}")
            model_context.save_model_state(save_path=save_path, tag=tag)

    save_path = os.path.join(traces.context().path, "checkpoints")
    model_context.train()
    train_iters = finetune_cfg['train_iters']
    overall_tokens_n = 0
    zero_rank = dist.get_rank() == 0
    t0 = time.time()

    train_ds = create_train_dataloader(
        jsonl_path=train_jsonl_path,
        model_name=model_context.model_name,
        encoding=model_context.encoding,
        num_workers=max(multiprocessing.cpu_count() // 2, 1),
        batch_size=finetune_cfg['train_batch_size'],
        ctx_size=finetune_cfg['model_info']['ctx_size']
    )
    train_ds_iter = iter(train_ds)
    test_ds = create_test_dataloader(
        jsonl_path=test_jsonl_path,
        model_name=model_context.model_name,
        encoding=model_context.encoding,
        ctx_size=finetune_cfg['model_info']['ctx_size']
    )
    test_ds = list(map(to_cuda, test_ds))

    early_stop = EarlyStopper(patience=int(train_iters * 0.2))
    with status_tracker(total_steps=train_iters) as stats_tracker:
        for iter_n in range(1, train_iters + 1):
            data = to_cuda(next(train_ds_iter))
            if zero_rank:
                traces.log(
                    f"iter {iter_n}/{finetune_cfg['train_iters']}  tokens {overall_tokens_n / 1e9:0.3f} "
                    f"input={traces.p(data['input'])}  mask={traces.p(data['mask'])} "
                    f"({data['mask'].sum()}/{data['mask'].numel()}) * {dist.get_world_size()} replicas"
                )
            train_loss, tokens_n = _train_iteration(
                data=data,
                iter_n=iter_n,
                model_context=model_context,
                finetune_cfg=finetune_cfg,
                status_tracker=status_tracker
            )
            overall_tokens_n += tokens_n * dist.get_world_size()

            test_loss = _test_iteration(
                test_ds=test_ds,
                iter_n=iter_n,
                model_context=model_context,
                finetune_cfg=finetune_cfg
            )

            stats_tracker.step(
                loss=train_loss,
                test_loss=test_loss,
                **{f'ds/{k}': v for k, v in data.get("stats", dict()).items()},
                **model_context.train_information(),
                gtokens=overall_tokens_n / 1e9,
                tokens_num=overall_tokens_n,
                time_elapsed=time.time() - t0,
            )

            if early_stop(test_loss):
                traces.log(f"Stopping the training due to "
                           f"test loss was above minimum {early_stop.counter} times")
                _save_checkpoint(force=True, iter_n=iter_n, loss=test_loss)
                break
            else:
                _save_checkpoint(force=False, iter_n=iter_n, loss=test_loss)


def main():
    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneStatusTracker()

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    try:
        status_tracker.update_status("working")
        _log_everywhere("Dest dir is %s" % traces.context().path)
        argv_copy = copy.deepcopy(sys.argv[1:])
        argv_copy = [x for x in argv_copy if not x.startswith("--local-rank")]  # --local-rank=5 is used by torch.distributed, ignore it
        finetune_cfg = _build_finetune_config_by_heuristics.main(argv_copy, standalone_mode=False)
        finetune_cfg = copy.deepcopy(finetune_cfg)

        _log_everywhere(f"Building the model {finetune_cfg['model_name']}")
        model_context = ModelContext(
            finetune_cfg=finetune_cfg,
            use_deepspeed=True
        )

        _log_everywhere(f"Starting finetune at {traces.context().path}\n\n")
        loop(
            train_jsonl_path=TRAIN_FILTERED_FILEPATH,
            test_jsonl_path=TEST_FILTERED_FILEPATH,
            finetune_cfg=finetune_cfg,
            model_context=model_context,
            status_tracker=status_tracker
        )

        _log_everywhere("finished finetune at %s" % traces.context().path)
        status_tracker.update_status("finished")

    # finetune_sequence relies on exit code to continue or stop
    except (SystemExit, KeyboardInterrupt):
        # caught sigusr1, interrupt by watchdog or by user
        # this has to be there, even if catch_sigusr1() already called exit with 99, otherwise exit code is zero
        exit(99)
    except Exception as e:
        _log_everywhere(f"Finetune has failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        raise e


if __name__ == "__main__":
    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    if "RANK" not in os.environ:
        os.environ["WORLD_SIZE"] = "1"
        os.environ["LOCAL_RANK"] = os.environ["RANK"] = "0"
        dist.init_process_group(backend='nccl', init_method="tcp://localhost:23456", world_size=1, rank=0)
    else:
        dist.init_process_group(backend='nccl', init_method="env://")
        th.cuda.set_device(dist.get_rank())
    main()
