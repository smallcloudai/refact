import sys
import click
import copy
import json
import shutil
import logging
import multiprocessing
import os
import signal
import time
import traceback
import filelock
import jsonlines
from pathlib import Path
from typing import Dict, Any, Iterable, Tuple

import torch as th
import torch.distributed as dist

from refact_utils.scripts import env
from refact_utils.finetune.utils import finetune_train_defaults
from refact_webgui.webgui.selfhost_static import safe_paths_join
from self_hosting_machinery.finetune.configuration.finetune_config import base_config, ConfigBuilder
from self_hosting_machinery.finetune.scripts.auxiliary.dataset import (
    create_train_dataloader, create_test_dataloader, get_ds_len_per_epoch, to_cuda, count_file_types
)
from self_hosting_machinery.finetune.scripts.auxiliary.early_stopper import EarlyStopper
from self_hosting_machinery.finetune.scripts.auxiliary.finetune_status_tracker import FinetuneStatusTracker
from self_hosting_machinery.finetune.scripts.auxiliary.model import ModelContext
from self_hosting_machinery.finetune.scripts import finetune_filter
from self_hosting_machinery.finetune.utils import traces


def _log_everywhere(message):
    if dist.get_rank() != 0:
        return
    logging.info(message)
    traces.log(message)


def _build_finetune_config_by_heuristics(run_id: str, finetune_cfg: Dict, model_config: Dict, **kwargs) -> Dict[str, Any]:
    user_cfg = copy.deepcopy(finetune_train_defaults)
    user_cfg_nondefault = {}
    for k, v in kwargs.items():
        # traces.log("Command line parameter: %s = %s" % (k, v))
        user_cfg[k] = v
        if finetune_train_defaults.get(k, 0) != v:
            user_cfg_nondefault[k] = v

    cfg_builder = ConfigBuilder(finetune_cfg)
    # if user_cfg['use_heuristics']:
    if user_cfg['train_steps'] == 0:
        _log_everywhere("Retrieving dataset length per epoch, it may take a while...")
        ds_len = get_ds_len_per_epoch(env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id), model_config, cfg_builder)
        traces.log(f"Dataset length per epoch = {ds_len}")
        # set_lora_quality_by_heuristics sets inside:
        # lora_target_modules=[
        #             "qkv", "out", "mlp",
        #         ], lora_r=64, lora_alpha=128, lora_dropout=0.01,
        #             freeze_exceptions=[
        #                 "wte", "lm_head", "lora"
        #             ]
        (cfg_builder
         .set_batch_size(cfg_builder.cfg['train_batch_size'])
        #  .set_lora_quality_by_heuristics(ds_len=ds_len, initial_loss=initial_loss)
         .set_schedule_by_heuristics(ds_len=ds_len)    # analog of set_train_steps + set_lr_decay_steps
         .set_lora_r(user_cfg['lora_r'])
         .set_lora_alpha(user_cfg['lora_alpha'])
         .set_lora_dropout(user_cfg['lora_dropout'])
         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
        #  .set_low_gpu_mem_mode_by_heuristics())
    else:
        _log_everywhere("Using finetune setup parameters")
        (cfg_builder
         .set_train_steps(user_cfg['train_steps'])
         .set_lr_decay_steps(max(user_cfg['lr_decay_steps'], user_cfg['train_steps']))
         .set_lora_r(user_cfg['lora_r'])
         .set_lora_alpha(user_cfg['lora_alpha'])
         .set_lora_dropout(user_cfg['lora_dropout'])
         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
    (cfg_builder
        .set_lr(user_cfg['lr'])
        .set_batch_size(user_cfg['batch_size'])
        .set_warmup_steps(user_cfg['warmup_num_steps'])
        # .set_limit_time_seconds(user_cfg['limit_time_seconds'])
        .set_weight_decay(user_cfg['weight_decay']))

    if dist.get_rank() == 0:
        filetypes_train = count_file_types(env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id))
        filetypes_test = count_file_types(env.PERRUN_TEST_FILTERED_FILEPATH(run_id))
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
                "run_id": run_id,
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


def convert_to_int(v):
    print(v, type(v))
    if isinstance(v, str):
        if v.endswith('.0'):
            v = v[:-2]
        try:
            return int(v)
        except ValueError:
            raise click.BadParameter('Value must be an integer')
    else:
        return v


def gpu_filter_and_build_config(
        pname: str,
        run_id: str,
        model_name: str,
        model_info: Dict[str, Any],
        model_config: Dict[str, Any],
        model_ctx_size: int,
        **kwargs) -> Dict[str, Any]:
    if model_ctx_size > 0:
        model_info["T"] = model_ctx_size
    finetune_cfg = {
        **base_config(model_name=model_name, model_info=model_info),
        **kwargs,
    }
    traces.log("locking \"%s\" for filtering" % pname)
    if dist.get_rank() == 0:
        with filelock.FileLock(env.PP_PROJECT_LOCK(pname)):
            traces.log("locked \"%s\" successfully" % pname)
            finetune_filter.finetune_gpu_filter(pname, finetune_cfg, model_config)
            traces.log("completed filtering, now copy files to run \"%s\"" % run_id)
            _copy_source_files(
                env.PP_TRAIN_FILTERED_FILEPATH(pname), env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id), pname, run_id)
            _copy_source_files(
                env.PP_TEST_FILTERED_FILEPATH(pname), env.PERRUN_TEST_FILTERED_FILEPATH(run_id), pname, run_id)
    else:
        finetune_filter.finetune_gpu_filter(pname, finetune_cfg, model_config)
    dist.barrier()

    return _build_finetune_config_by_heuristics(run_id, finetune_cfg, model_config, **kwargs)


def _copy_source_files(jsonl_src, jsonl_dst, pname, run_id):
    for d in jsonlines.open(jsonl_src):
        print(d["path"])
        try:
            src_path = safe_paths_join(env.PP_DIR_UNPACKED(pname), d["path"])
            dst_path = safe_paths_join(env.PERRUN_DIR_UNPACKED(run_id), d["path"])
        except ValueError as e:
            raise ValueError(f'copy source files error: {e}')
        os.makedirs(os.path.dirname(dst_path), exist_ok=True)
        shutil.copyfile(src_path, dst_path)
    os.makedirs(os.path.dirname(jsonl_dst), exist_ok=True)   # needed when zero files (edge case)
    shutil.copyfile(jsonl_src, jsonl_dst)


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
    status_tracker: FinetuneStatusTracker,
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
        model_config=model_context.model_mappings_config,
        encoding=model_context.encoding,
        num_workers=max(multiprocessing.cpu_count() // 2, 1),
        batch_size=finetune_cfg['train_batch_size'],
        ctx_size=finetune_cfg['model_info']['ctx_size']
    )
    train_ds_iter = iter(train_ds)
    test_ds = create_test_dataloader(
        jsonl_path=test_jsonl_path,
        model_config=model_context.model_mappings_config,
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


def main(supported_models: Dict[str, Any], models_db: Dict[str, Any]):
    args = parse_args()

    traces.configure(task_dir="loras", task_name=args.run_id, work_dir=env.PERMDIR)
    if "RANK" not in os.environ:
        os.environ["WORLD_SIZE"] = "1"
        os.environ["RANK"] = "0"
        os.environ["LOCAL_RANK"] = "0"
        port = localhost_port_not_in_use(21000, 22000)  # multi gpu training uses [20000, 21000) range
        dist.init_process_group(backend='nccl', init_method=f"tcp://localhost:{port}", world_size=1, rank=0)
    else:
        dist.init_process_group(backend='nccl', init_method='env://')
    th.cuda.set_device(dist.get_rank())

    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneStatusTracker()

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    try:
        assert args.model_name in models_db, f"unknown model '{args.model_name}'"
        assert args.model_name in supported_models, f"model '{args.model_name}' not in finetune supported_models"
        model_config = supported_models[args.model_name]
        model_info = models_db[args.model_name]
        assert "finetune" in model_info.get("filter_caps", []), f"model {args.model_name} does not support finetune"

        status_tracker.update_status("working")
        _log_everywhere("Dest dir is %s" % traces.context().path)

        finetune_cfg = gpu_filter_and_build_config(model_config=model_config, model_info=model_info, **vars(args))

        _log_everywhere(f"Building the model {finetune_cfg['model_name']}")
        model_context = ModelContext(
            finetune_cfg=finetune_cfg,
            model_config=model_config,
            use_deepspeed=True
        )

        _log_everywhere(f"Starting finetune at {traces.context().path}\n\n")
        loop(
            train_jsonl_path=env.PERRUN_TRAIN_FILTERED_FILEPATH(args.run_id),
            test_jsonl_path=env.PERRUN_TEST_FILTERED_FILEPATH(args.run_id),
            finetune_cfg=finetune_cfg,
            model_context=model_context,
            status_tracker=status_tracker,
        )

        _log_everywhere("finished finetune at %s" % traces.context().path)
        status_tracker.update_status("finished")

    # finetune_sequence relies on exit code to continue or stop
    except (SystemExit, KeyboardInterrupt):
        # caught sigusr1, interrupt by watchdog or by user
        # this has to be there, even if catch_sigusr1() already called exit with 99, otherwise exit code is zero
        exit(99)
    except Exception as e:
        traces.log(traceback.format_exc())
        _log_everywhere(f"Finetune has failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        raise e


def localhost_port_not_in_use(start: int, stop: int):
    def _is_port_in_use(port: int) -> bool:
        import socket
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            return s.connect_ex(('localhost', port)) == 0

    for port in range(start, stop):
        if not _is_port_in_use(port):
            return port

    raise RuntimeError(f"cannot find port in range [{start}, {stop})")


def parse_args():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument('--pname', type=str, required=True)
    parser.add_argument('--run_id', type=str, required=True)
    parser.add_argument('--model_name', type=str, required=True)
    parser.add_argument('--trainable_embeddings', default=finetune_train_defaults['trainable_embeddings'])
    parser.add_argument('--low_gpu_mem_mode', default=finetune_train_defaults['low_gpu_mem_mode'])
    parser.add_argument('--lr', type=float, default=finetune_train_defaults['lr'])
    parser.add_argument('--batch_size', type=int, default=finetune_train_defaults['batch_size'])
    parser.add_argument('--warmup_num_steps', type=int, default=finetune_train_defaults['warmup_num_steps'])
    parser.add_argument('--weight_decay', type=float, default=finetune_train_defaults['weight_decay'])
    parser.add_argument('--train_steps', type=int, default=finetune_train_defaults['train_steps'])
    parser.add_argument('--lr_decay_steps', type=int, default=finetune_train_defaults['lr_decay_steps'])
    parser.add_argument('--lora_r', type=int, default=finetune_train_defaults['lora_r'], choices=[4, 8, 16, 32, 64])
    parser.add_argument('--lora_alpha', type=int, default=finetune_train_defaults['lora_alpha'], choices=[4, 8, 16, 32, 64, 128])
    parser.add_argument('--lora_dropout', type=float, default=finetune_train_defaults['lora_dropout'])
    parser.add_argument('--model_ctx_size', type=int, default=finetune_train_defaults['model_ctx_size'])
    parser.add_argument('--filter_loss_threshold', type=float, default=finetune_train_defaults['filter_loss_threshold'])
    parser.add_argument("--local-rank", type=int, default=0)  # is used by torch.distributed, ignore it

    return parser.parse_args()


if __name__ == "__main__":
    from refact_known_models import models_mini_db
    from self_hosting_machinery.finetune.configuration import supported_models

    main(supported_models=supported_models.config, models_db=models_mini_db)
