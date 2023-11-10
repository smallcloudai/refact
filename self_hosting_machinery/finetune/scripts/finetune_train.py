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

from self_hosting_machinery import env
from self_hosting_machinery.finetune.configuration.finetune_config import base_config, ConfigBuilder
from self_hosting_machinery.finetune.scripts.aux.dataset import (
    create_train_dataloader, create_test_dataloader, get_ds_len_per_epoch, to_cuda
)
from self_hosting_machinery.finetune.scripts.aux.early_stopper import EarlyStopper
from self_hosting_machinery.finetune.scripts.aux.finetune_status_tracker import FinetuneStatusTracker
from self_hosting_machinery.finetune.scripts.aux.model import ModelContext
from self_hosting_machinery.finetune.utils import traces
from self_hosting_machinery.finetune.utils.finetune_utils import get_finetune_config


def _log_everywhere(message):
    logging.info(message)
    traces.log(message)


def _build_finetune_config_by_heuristics(models_db: Dict[str, Any]) -> Dict[str, Any]:
    with open(env.CONFIG_FINETUNE_FILTER_STAT, 'r') as f:
        initial_loss = json.load(f)["avg_loss"]

    user_cfg = get_finetune_config(models_db, logger=traces.log)
    cfg_builder = ConfigBuilder(base_config(user_cfg['model_name'], models_db))
    if user_cfg['use_heuristics']:
        _log_everywhere("Calculating finetune optimal parameters")
        _log_everywhere("Retrieving dataset length per epoch, it may take a while...")
        ds_len = get_ds_len_per_epoch(user_cfg['model_name'], cfg_builder)
        traces.log(f"Dataset length per epoch = {ds_len}")
        (cfg_builder
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
         .set_lora_init_scale(user_cfg['lora_init_scale'])
         .set_lora_dropout(user_cfg['lora_dropout'])
         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
        (cfg_builder
         .set_lr(user_cfg['lr'])
         .set_batch_size(user_cfg['batch_size'])
         .set_warmup_steps(user_cfg['warmup_num_steps'])
         .set_limit_time_seconds(user_cfg['limit_time_seconds'])
         .set_weight_decay(user_cfg['weight_decay']))

    traces.log(f'Freeze exceptions: {cfg_builder.cfg["model_info"]["freeze_exceptions"]}')
    traces.log(f'Low memory mode: {user_cfg["low_gpu_mem_mode"]}')
    for k, v in cfg_builder.cfg["model_info"]["lora"].items():
        traces.log(f'Lora config: {k:>20} {v}')

    with open(os.path.join(traces.context().path, "config.json"), "w") as f:
        json.dump(cfg_builder.cfg, f, indent=4)

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
    world_size = int(os.environ.get('WORLD_SIZE', 1))

    if finetune_cfg['debug']:
        data_path = Path(traces.context().path) / ('debug_data/iter%04d' % iter_n)
        data_path.mkdir(exist_ok=True, parents=True)

    losses, tokens_n = [], 0
    for b0 in range(0, finetune_cfg["train_batch_size"], finetune_cfg["micro_batch_size"]):
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

        if finetune_cfg['debug']:
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
        finetune_cfg: Dict[str, Any],
        model_context: ModelContext,
        status_tracker: FinetuneStatusTracker
):
    def _save_checkpoint(iter_n: int, loss: float, force: bool = False):
        if force or (iter_n != 0 and iter_n % finetune_cfg['save_every'] == 0):
            tag = f"iter{iter_n:04d}-testloss{loss:.3f}"
            traces.log(f"Saving checkpoint {tag}")
            model_context.save_model_state(save_path=save_path, tag=tag)

    save_path = os.path.join(traces.context().path, "checkpoints")
    model_context.train()
    train_iters = finetune_cfg['train_iters']
    overall_tokens_n = 0
    t0 = time.time()

    train_ds = create_train_dataloader(
        model_name=model_context.model_name,
        encoding=model_context.encoding,
        num_workers=max(multiprocessing.cpu_count() // 2, 1),
        batch_size=finetune_cfg['train_batch_size'],
        ctx_size=finetune_cfg['model_info']['ctx_size']
    )
    train_ds_iter = iter(train_ds)
    test_ds = create_test_dataloader(
        model_name=model_context.model_name,
        encoding=model_context.encoding,
        ctx_size=finetune_cfg['model_info']['ctx_size']
    )
    test_ds = list(map(to_cuda, test_ds))

    early_stop = EarlyStopper(patience=int(train_iters * 0.2))
    with status_tracker(total_steps=train_iters) as stats_tracker:
        for iter_n in range(1, train_iters + 1):
            data = to_cuda(next(train_ds_iter))
            traces.log(
                f"iter {iter_n}/{finetune_cfg['train_iters']}  tokens {overall_tokens_n / 1e9:0.3f} "
                f"input={traces.p(data['input'])}  mask={traces.p(data['mask'])} "
                f"({data['mask'].sum()}/{data['mask'].numel()})"
            )
            train_loss, tokens_n = _train_iteration(
                data=data,
                iter_n=iter_n,
                model_context=model_context,
                finetune_cfg=finetune_cfg,
                status_tracker=status_tracker
            )
            overall_tokens_n += tokens_n

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


def main(models_db: Dict[str, Any]):
    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneStatusTracker()

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    try:
        status_tracker.update_status("working")
        _log_everywhere("Loading finetune configs...")
        finetune_cfg = copy.deepcopy(_build_finetune_config_by_heuristics(models_db))

        _log_everywhere(f"Building the model {finetune_cfg['model_name']}")
        model_context = ModelContext(
            finetune_cfg=finetune_cfg,
            use_deepspeed=True
        )

        _log_everywhere(f"Starting finetune at {traces.context().path}\n\n")
        loop(
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
        _log_everywhere(f"Finetune is failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        raise e


if __name__ == "__main__":
    from known_models_db.refact_known_models import models_mini_db

    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    main(models_mini_db)
