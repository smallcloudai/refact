import copy
import json
import logging
import os
import signal
import subprocess
import sys
import time
from functools import partial
from pathlib import Path
from typing import Optional, Dict, Any

import torch as th

from refact_data_pipeline.datautils import BatchIterator
from self_hosting_machinery import env
from self_hosting_machinery.finetune.configuration import supported_models
from self_hosting_machinery.finetune.configuration.finetune_config import base_config, ConfigBuilder
from self_hosting_machinery.finetune.modelling.model_handling import make_model, save_model_state, model_forward
from self_hosting_machinery.finetune.scripts.script_aux.dataset_context import get_ds_len_per_epoch
from self_hosting_machinery.finetune.scripts.script_aux.early_stopper import EarlyStopper
from self_hosting_machinery.finetune.scripts.script_aux.finetune_status_tracker import FinetuneStatusTracker
from self_hosting_machinery.finetune.scripts.script_aux.model import ModelContext
from self_hosting_machinery.finetune.utils import traces
from self_hosting_machinery.finetune.utils.finetune_utils import get_finetune_config


def _log_everywhere(message):
    logging.info(message)
    traces.log(message)


def save_status_json(status_dict, status_string):
    # FIXME: rank == 0
    rank = 0
    if rank != 0:
        return
    traces.touch()
    env.report_status("ftune", status_string)
    status_dict["status"] = status_string
    if not traces.context():
        return
    try:
        with open(os.path.join(traces.context().path, "status.json.tmp"), "w") as f:
            json.dump(status_dict, f, indent=4)
        os.rename(os.path.join(traces.context().path, "status.json.tmp"),
                  os.path.join(traces.context().path, "status.json"))
    except Exception as e:
        traces.log("ERROR SAVING STATS: %s" % (e or str(type(e))))
        traces.log("(no big deal, will try again next iteration)")


def build_finetune_config_by_heuristics(models_db: Dict[str, Any]) -> Dict[str, Any]:
    with open(env.CONFIG_FINETUNE_FILTER_STAT, 'r') as f:
        initial_loss = json.load(f)["avg_loss"]

    user_cfg = get_finetune_config(models_db, logger=traces.log)
    cfg_builder = ConfigBuilder(base_config(user_cfg['model_name'], models_db))
    if user_cfg['use_heuristics']:
        traces.log("Retrieving dataset length per epoch, it may take a while...")
        ds_len = get_ds_len_per_epoch(user_cfg['model_name'], cfg_builder)
        traces.log(f"Dataset length per epoch = {ds_len}")
        (cfg_builder
         .set_lora_quality_by_heuristics(ds_len=ds_len, initial_loss=initial_loss)
         .set_schedule_by_heuristics(ds_len=ds_len)
         .set_low_gpu_mem_mode_by_heuristics())
    else:
        (cfg_builder
         .set_train_steps(user_cfg['train_steps'])
         .set_lr_decay_steps(user_cfg['lr_decay_steps'])
         .set_lora_r(user_cfg['lora_r'])
         .set_lora_alpha(user_cfg['lora_alpha'])
         .set_lora_init_scale(user_cfg['lora_init_scale'])
         .set_lora_dropout(user_cfg['lora_dropout'])
         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode']))
        (cfg_builder
         .set_lr(user_cfg['lr'])
         .set_batch_size(user_cfg['batch_size'])
         .set_warmup_steps(user_cfg['warmup_num_steps'])
         .set_limit_time_seconds(user_cfg['limit_time_seconds'])
         .set_weight_decay(user_cfg['weight_decay']))

    traces.log(f'Freeze exceptions: {cfg_builder.cfg["model_info"]["freeze_exceptions"]}')
    for k, v in cfg_builder.cfg["model_info"]["lora"].items():
        traces.log(f'Lora config: {k:>20} {v}')

    with open(os.path.join(traces.context().path, "config.json"), "w") as f:
        json.dump(cfg_builder.cfg, f, indent=4)

    return cfg_builder.cfg


def loop(
        finetune_cfg: Dict[str, Any],
        model_context: ModelContext,
        status_tracker: FinetuneStatusTracker
):
    def _save_checkpoint(force: bool = False):
        if force or (iter_n != 0 and iter_n % cfg['save_every'] == 0):
            if "test_loss" in progress:
                tag = "iter%04d-testloss%0.3f" % (iter_n, progress["test_loss"])
            else:
                tag = "iter%04d-trainloss%0.3f" % (iter_n, progress["loss"])
            traces.log("saving checkpoint %s" % tag)
            save_model_state(model, save_path=save_path, tag=tag)

    model_config = supported_models.config[model_name]
    save_path = os.path.join(traces.context().path, "checkpoints")
    model_context.train()
    test_ds_fn = partial(BatchIterator, dataopts=dict(
        batch_size=1,
        drop_last=False
    ))
    micro_bs = cfg['micro_batch_size']
    tokens_n = 0
    iter_time_last = None
    t0 = time.time()
    # Each checkpoint must be tested:
    assert cfg['train_iters'] % cfg['test_every'] == 0
    assert cfg['save_every'] % cfg['test_every'] == 0
    plot_process: Optional[subprocess.Popen] = None
    save_status_json(status_dict, "working")
    low_gpu_mem_mode = cfg['low_gpu_mem_mode'] or model_config['force_enable_checkpointing']
    forward = partial(model_forward, model=model)
    early_stop = EarlyStopper(patience=int(cfg['train_iters'] * 0.2))
    for iter_n in range(cfg['train_iters'] + 1):  # +1 so we can save 100 (not 99)
        t0_iter = time.time()
        traces.progress("iteration", iter_n)
        data = next(train_ds, None)
        if data is None:
            break
        batch, ds_stats = data

        if cfg['debug']:
            data_path = Path(traces.context().path) / ('debug_data/iter%04d' % iter_n)
            data_path.mkdir(exist_ok=True, parents=True)
        traces.log(
            f"iter {iter_n}/{cfg['train_iters']}  tokens {tokens_n / 1e9:0.3f} "
            f"input={traces.p(batch['input'])}  mask={traces.p(batch['mask'])} "
            f"({batch['mask'].sum()}/{batch['mask'].numel()})"
        )

        for b0 in range(0, cfg.get("train_batch_size"), cfg.get("micro_batch_size")):
            try:
                input = batch['input'][b0:b0 + micro_bs].contiguous()
                logits = forward(input=input, low_gpu_mem_mode=low_gpu_mem_mode)
                loss = loss_function(
                    logits=logits,
                    labels=batch['labels'][b0:b0 + micro_bs].contiguous(),
                    mask=batch['mask'][b0:b0 + micro_bs].contiguous(),
                )
                model.backward(loss)
            except th.cuda.OutOfMemoryError as e:
                if low_gpu_mem_mode:
                    raise e
                else:
                    model.optimizer.zero_grad()
                    th.cuda.empty_cache()
                    low_gpu_mem_mode = True
                    traces.log("switching to low GPU memory mode")
                    continue

            model.step()
            tokens_n += input.shape[0] * input.shape[1]
            traces.progress('loss', loss)

            if cfg['debug']:
                with open(data_path / ('%d_%0.3f.txt' % (b0, loss.item())), 'w') as f:
                    f.write(model.encoding.decode(input[0].cpu().numpy()))

        if test_ds is not None and cfg["test_every"] > 0 and iter_n % cfg["test_every"] == 0:
            model.eval()
            with th.inference_mode():
                test_losses = []
                for batch, _ in test_ds_fn(test_ds):
                    logits = forward(input=batch['input'], low_gpu_mem_mode=low_gpu_mem_mode)
                    test_loss = loss_function(
                        logits=logits,
                        labels=batch['labels'],
                        mask=batch['mask'],
                    )
                    traces.progress('test_loss', test_loss)
                    test_losses.append(test_loss)
                if len(test_losses) > 0 and early_stop(sum(test_losses) / len(test_losses)):
                    traces.log(f"Stopping the training due to "
                               f"test loss was above minimum {early_stop.counter} times")

                    _save_checkpoint(iter_n=iter_n, loss=loss)
                    break
            model_context.train()

        for k, v in ds_stats.items():
            traces.progress(f'ds/{k}', v)
        traces.progress("gtokens", tokens_n / 1e9)
        traces.progress("lr", optimizer.param_groups[-1]['lr'])
        traces.progress("gpumem_p0", th.cuda.max_memory_allocated())
        traces.progress("num_skipped_updates", model.skipped_steps)
        traces.progress("scale", model.optimizer.cur_scale)
        traces.progress("tokens_num", tokens_n)
        traces.progress("time_elapsed", time.time() - t0)
        iter_time = time.time() - t0_iter
        if iter_time_last is None:
            eta = (cfg['train_iters'] + 1 - iter_n) * iter_time
        else:
            eta = (cfg['train_iters'] + 1 - iter_n) * ((iter_time + iter_time_last) / 2)
        traces.progress("eta_minutes", int(round(eta / 60)))
        iter_time_last = iter_time
        progress = traces.progress_dump(step=iter_n)

        if plot_process is not None:
            plot_process.communicate()
        plot_process = subprocess.Popen([
            sys.executable,
            os.path.join(os.path.dirname(__file__), "../utils/traces_plot.py"),
            "progress.jsonl",
            "%d" % (cfg['train_iters'] + 50),
        ], cwd=traces.context().path)
        _save_checkpoint(force=False)
        status_dict["worked_steps"] = iter_n
        status_dict["worked_minutes"] = int((time.time() - t0) / 60)
        status_dict["eta_minutes"] = int(round(eta / 60))
        save_status_json(status_dict, "working")
        if "test_loss" in progress:
            logging.info("finished iteration %d, train_loss=%0.3f, test_loss=%0.3f"
                         % (iter_n, progress["loss"], progress["test_loss"]))
        else:
            logging.info("finished iteration %d, train_loss=%0.3f" % (iter_n, progress["loss"]))


def main(models_db: Dict[str, Any]):
    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneStatusTracker()

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    _log_everywhere("Loading finetune configs...")
    finetune_cfg = build_finetune_config_by_heuristics(models_db)
    model_cfg = copy.deepcopy(base_config(finetune_cfg["model_name"], models_db))

    try:
        status_tracker.update_status("working")
        _log_everywhere(f"Starting finetune at {traces.context().path}\n\n"
                        f"Building the model...")
        model_context = ModelContext(
            finetune_cfg=finetune_cfg,
            model_cfg=model_cfg,
            use_deepspeed=True
        )
        loop(
            finetune_cfg=finetune_cfg,
            model_context=model_context,
            status_tracker=status_tracker
        )
        logging.info("finished finetune at %s" % traces.context().path)
        status_tracker.update_status("finished")

    # finetune_sequence relies on exit code to continue or stop
    except (SystemExit, KeyboardInterrupt):
        # caught sigusr1, interrupt by watchdog or by user
        # this has to be there, even if catch_sigusr1() already called exit with 99, otherwise exit code is zero
        exit(99)
    except Exception as e:
        _log_everywhere(f"Finetune is failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        exit(1)


if __name__ == "__main__":
    from known_models_db.refact_known_models import models_mini_db

    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    main(models_mini_db)
