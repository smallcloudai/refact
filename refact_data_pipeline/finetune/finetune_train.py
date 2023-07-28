import os
import time
import json
import subprocess
import sys
import signal

import deepspeed
import logging
import torch as th

from functools import partial
from pathlib import Path
from jsonlines import jsonlines
from torchinfo import summary

from refact_encoding import RefactEncoding
from refact_models.checkpoint_loader import load_config
from refact_data_pipeline.finetune import traces
from refact_data_pipeline import DatasetOpts, finetune_datasource
from refact_data_pipeline.datautils import BatchIterator
from refact_data_pipeline.finetune.finetune_config import base_config, ConfigBuilder
from refact_data_pipeline.finetune.finetune_train_defaults import finetune_train_defaults
from refact_data_pipeline.finetune.model_handling import make_model, masked_loss, save_model_state
from self_hosting_machinery import env

from typing import Optional, Callable, Dict, Any, Tuple


filtered_train = "train_set_filtered.jsonl"
filtered_test = "test_set_filtered.jsonl"


def save_status_json(status_dict, status_string):
    # FIXME: rank == 0
    rank = 0
    if rank != 0:
        return
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


def load_finetune_config() -> Dict[str, Any]:
    def _get_ds_len_per_epoch(cfg_builder):
        ds_opts = DatasetOpts(f"n_ctx={cfg_builder.cfg['model_info']['ctx_size'] + 1},"
                              f"pack_at_most=1,quit_on_epoch=1,seed=42")
        ds_opts.set_encoding(RefactEncoding(
            load_config(root_path=cfg_builder.cfg['model_info']['weight_path'],
                        repo_id=cfg_builder.cfg['model_info']['repo_id']).enc_name))
        ds = finetune_datasource.local_mix_plain_infill(filtered_train, ds_opts)
        ds_len = 0
        try:
            for _ in ds:
                ds_len += 1
            return ds_len
        except:
            return ds_len

    with open(env.CONFIG_FINETUNE_FILTER_STATS, 'r') as f:
        initial_loss = json.load(f)["avg_loss"]
    cfg_builder = ConfigBuilder(base_config(env))
    user_cfg = {**finetune_train_defaults}
    if os.path.exists(env.CONFIG_FINETUNE):
        traces.log("Reading %s" % env.CONFIG_FINETUNE)
        user_cfg.update(**json.load(open(env.CONFIG_FINETUNE)))
    if user_cfg['use_heuristics']:
        traces.log("Retrieving dataset length per epoch, it may take a while...")
        ds_len = _get_ds_len_per_epoch(cfg_builder)
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


def create_data(cfg, enc) -> Tuple[Any, Optional[Any]]:
    train_dataopts = DatasetOpts("n_ctx=%d,pack_at_most=10,shuffle_depth=3000" % (cfg['model_info']['ctx_size'] + 1))
    train_dataopts.set_encoding(enc)
    test_dataopts = DatasetOpts("n_ctx=%d,pack_at_most=1,quit_on_epoch=1,seed=42" % (cfg['model_info']['ctx_size'] + 1))
    test_dataopts.set_encoding(enc)
    train_ds = finetune_datasource.local_mix_plain_infill(filtered_train, train_dataopts)
    train_ds = BatchIterator(train_ds, dataopts=dict(
        batch_size=cfg['train_batch_size'],
        drop_last=True
    ))

    if os.path.exists(os.path.join(env.DIR_UNPACKED, filtered_test)) and \
            len(list(jsonlines.open(os.path.join(env.DIR_UNPACKED, filtered_test)))) > 0:
        test_ds = finetune_datasource.local_sequence_plain_infill(filtered_test, test_dataopts)
        test_ds = list(test_ds)
    else:
        traces.log("Warning: no test set has been provided")
        test_ds = None
    return train_ds, test_ds


def loop(
        cfg,
        model,
        optimizer,
        loss_function: Callable,
        *,
        status_dict,
        train_ds,
        test_ds: Optional[Any]
):
    save_path = os.path.join(traces.context().path, "checkpoints")
    model.train()
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
    save_status_json(status_dict, "starting")
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
            "iter %i/%i  tokens %0.3fG  input=%s  mask=%s (%i/%i)" % (
                iter_n, cfg['train_iters'], tokens_n / 1e9, traces.p(batch['input']), traces.p(batch["mask"]),
                batch["mask"].sum(), batch["mask"].numel()
            )
        )

        for b0 in range(0, cfg.get("train_batch_size"), cfg.get("micro_batch_size")):
            input = batch['input'][b0:b0 + micro_bs].contiguous()
            if cfg['low_gpu_mem_mode']:
                logits = model.forward_train_cp(input)
            else:
                logits = model.lm_forward(model(input, attention_mask=None)[0])

            loss = loss_function(
                logits=logits,
                labels=batch['labels'][b0:b0 + micro_bs].contiguous(),
                mask=batch['mask'][b0:b0 + micro_bs].contiguous(),
            )
            model.backward(loss)
            model.step()
            tokens_n += input.shape[0] * input.shape[1]
            traces.progress('loss', loss)

            if cfg['debug']:
                with open(data_path / ('%d_%0.3f.txt' % (b0, loss.item())), 'w') as f:
                    f.write(model.encoding.decode(input[0].cpu().numpy()))

        if test_ds is not None and cfg["test_every"] > 0 and iter_n % cfg["test_every"] == 0:
            model.eval()
            with th.inference_mode():
                for batch, _ in test_ds_fn(test_ds):
                    logits = model.lm_forward(model(batch['input'], attention_mask=None)[0])
                    loss = loss_function(
                        logits=logits,
                        labels=batch['labels'],
                        mask=batch['mask'],
                    )
                    traces.progress('test_loss', loss)
            model.train()

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
            os.path.join(os.path.dirname(__file__), "traces_plot.py"),
            "progress.jsonl",
            "%d" % (cfg['train_iters'] + 50),
        ], cwd=traces.context().path)
        if iter_n != 0 and iter_n % cfg['save_every'] == 0:
            if "test_loss" in progress:
                tag = "iter%04d-testloss%0.3f" % (iter_n, progress["test_loss"])
            else:
                tag = "iter%04d-trainloss%0.3f" % (iter_n, progress["loss"])
            traces.log("saving checkpoint %s" % tag)
            save_model_state(model, save_path=save_path, tag=tag)
        status_dict["worked_steps"] = iter_n
        status_dict["worked_minutes"] = int((time.time() - t0) / 60)
        status_dict["eta_minutes"] = int(round(eta / 60))
        save_status_json(status_dict, "working")
        if "test_loss" in progress:
            logging.info("finished iteration %d, train_loss=%0.3f, test_loss=%0.3f"
                         % (iter_n, progress["loss"], progress["test_loss"]))
        else:
            logging.info("finished iteration %d, train_loss=%0.3f" % (iter_n, progress["loss"]))


def finetune(status_dict):
    logging.info("starting finetune at %s" % traces.context().path)
    logging.info("STATUS finetune init")
    cfg = load_finetune_config()
    traces.log("creating model...")
    t0 = time.time()
    model = make_model(
        weights_path=cfg['model_info']['weight_path'],
        repo_id=cfg['model_info']['repo_id'],
        freeze_exceptions=cfg['model_info']['freeze_exceptions'],
        lora_target_modules=cfg['model_info']['lora']['lora_target_modules'],
        lora_r=cfg['model_info']['lora']['lora_r'],
        lora_alpha=cfg['model_info']['lora']['lora_alpha'],
        lora_dropout=cfg['model_info']['lora']['lora_dropout'],
        lora_init_scale=cfg['model_info']['lora']['lora_init_scale'],
        dtype=th.bfloat16 if 'bf16' in cfg and cfg['bf16']['enabled'] else th.float16,
        init_device="cuda",
        device="cuda",
    )
    t1 = time.time()
    traces.log("/model %0.1fms" % ((t1 - t0) * 1000))
    if cfg['debug']:
        summary(model, depth=4, col_names=['num_params', 'params_percent', 'trainable'])
    model, optimizer, _, _ = deepspeed.initialize(
        config=cfg,
        model=model,
        model_parameters=[p for p in model.parameters() if p.requires_grad],
        dist_init_required=True
    )
    train_ds, test_ds = create_data(cfg, model.encoding)
    logging.info("STATUS finetune working")
    loop(
        cfg=cfg,
        model=model,
        optimizer=optimizer,
        loss_function=partial(
            masked_loss, average_elements=cfg['model_info']['loss_average_elements'],
            enc=model.encoding
        ),
        train_ds=train_ds,
        test_ds=test_ds,
        status_dict=status_dict
    )
    logging.info("finished finetune at %s" % traces.context().path)


def main():
    status_dict = {
        "started_ts": time.time(),
        "worked_steps": 0,
        "worked_minutes": 0,
        "status": "starting",
        "quality": "unknown"
    }
    save_status_json(status_dict, "starting")

    def catch_sigusr1(signum, frame):
        logging.error("Interrupted: caught SIGUSR1")
        traces.log("Interrupted")
        save_status_json(status_dict, "interrupted")
        sys.exit(1)

    signal.signal(signal.SIGUSR1, catch_sigusr1)
    try:
        save_status_json(status_dict, "working")
        finetune(status_dict)
        save_status_json(status_dict, "finished")
    except BaseException as e:  # BaseException includes KeyboardInterrupt
        if "error" not in status_dict:  # if there is, a more detailed error is already in place
            t = str(e) or str(type(e))
            status_dict["error"] = t
            traces.log("FAILED: %s" % t)
            save_status_json(status_dict, "failed")
        logging.error("FAILED finetune at %s" % traces.context().path)
        logging.error("Error was: %s" % status_dict["error"])
        raise e


if __name__ == "__main__":
    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    main()
