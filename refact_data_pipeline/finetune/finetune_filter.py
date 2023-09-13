import math
import os
import time
import json
import random

import jsonlines
import textwrap
import sys
import signal
import logging
import torch as th

from functools import partial
from torchinfo import summary

import refact_data_pipeline.finetune.traces as traces
from refact_data_pipeline import DatasetOpts, finetune_datasource
from refact_data_pipeline.datautils import BatchIterator
from refact_data_pipeline.finetune.finetune_utils import get_finetune_config
from refact_data_pipeline.finetune.finetune_utils import get_finetune_filter_stats
from refact_data_pipeline.finetune.finetune_filtering_defaults import finetune_filtering_defaults
from refact_data_pipeline.finetune.finetune_config import base_config
from refact_data_pipeline.finetune.model_handling import make_model, masked_loss, model_forward
from refact_data_pipeline.finetune.process_uploaded_files import make_matcher
from self_hosting_machinery import env

from typing import List, Dict, Any


unfiltered_train = os.path.join(env.DIR_UNPACKED, "train_set.jsonl")
unfiltered_test = os.path.join(env.DIR_UNPACKED, "test_set.jsonl")

filtered_train = os.path.join(env.DIR_UNPACKED, "train_set_filtered.jsonl")
filtered_test = os.path.join(env.DIR_UNPACKED, "test_set_filtered.jsonl")


def _update_and_dump_status(status_dict: Dict[str, Any], status_string: str):
    if status_string in ["starting"]:
        status_dict = get_finetune_filter_stats(default=True)
        status_dict["started_ts"] = time.time()
    status_dict["status"] = status_string
    with open(env.CONFIG_FINETUNE_FILTER_STATS + ".tmp", "w") as f:
        json.dump(status_dict, f, indent=4)
    os.rename(env.CONFIG_FINETUNE_FILTER_STATS + ".tmp", env.CONFIG_FINETUNE_FILTER_STATS)
    return status_dict


def _file_accepted(reason, path):
    with open(env.LOG_FILES_ACCEPTED_FTF, "a") as f:
        f.write("%s %s\n" % (reason, path))


def _file_rejected(reason, path):
    with open(env.LOG_FILES_REJECTED_FTF, "a") as f:
        f.write("%s %s\n" % (reason, path))


def get_force_included_excluded_matchers():
    fcfg = {
        "filetypes_finetune": {},
        "filetypes_db": {}
    }
    if os.path.exists(env.CONFIG_HOW_TO_FILETYPES):
        traces.log("Reading %s" % env.CONFIG_HOW_TO_FILETYPES)
        with open(env.CONFIG_HOW_TO_FILETYPES, "r") as f:
            fcfg.update(**json.load(f))

    force_include_matcher, _ = make_matcher(fcfg.get('force_include', ''))
    force_exclude_matcher, _ = make_matcher(fcfg.get('force_exclude', ''))

    return force_include_matcher, force_exclude_matcher


@th.inference_mode()
def loss_based_filter(
        train_files: List,
        model,
        loss_function,
        dataopts,
        *,
        fcfg,
        status_dict,
        cfg,
):
    t0 = time.time()
    iter_times = []
    model.eval()
    batch_iter_fn = partial(BatchIterator, dataopts=dict(batch_size=1, drop_last=False))
    all_losses, rejected = [], set()
    logging.info("STATUS filtering")
    status_dict['total_steps'] = len(train_files)
    is_force_included, is_force_excluded = get_force_included_excluded_matchers()
    forward = partial(model_forward, model=model, low_gpu_mem_mode=False, backend=cfg['model_info']['backend'])
    for iter_n, file in enumerate(train_files):
        t0_iter = time.time()
        status_dict = _update_and_dump_status(status_dict, "filtering")
        file_losses = []
        if is_force_included(file['path']):
            _file_accepted("FILTER1 INCLUDED_BY_MASK", file["path"])
            status_dict["accepted"] += 1
            continue
        elif is_force_excluded(file['path']):
            traces.log("REJECTED FILTER %-100s EXCLUDED_BY_MASK" % file["path"])
            rejected.add(file["path"])
            _file_rejected("FILTER1 EXCLUDED_BY_MASK", file["path"])
            status_dict["rejected"] += 1
            continue

        for batch, stats in batch_iter_fn(finetune_datasource.local_plain([file], dataopts)):
            logits = forward(input=batch['input'])
            loss = float(loss_function(
                logits=logits.to(th.bfloat16),  # more stable than float16 and takes much less memory than float32
                labels=batch['labels'],
                mask=batch['mask'],
            ).item())
            if math.isnan(loss) or math.isinf(loss):
                traces.log(f"Skipping invalid loss={loss:.2f} value in file {file['path']}")
            else:
                file_losses.append(loss)

        if len(file_losses) == 0:
            traces.log("REJECTED FILTER %-100s empty" % file["path"])
            rejected.add(file["path"])
            _file_rejected("FILTER1 EMPTY", file["path"])
            status_dict["rejected"] += 1
            continue

        file_loss = sum(file_losses) / len(file_losses)

        if file_loss > fcfg['filter_loss_threshold']:
            traces.log("REJECTED FILTER %-100s loss %0.3f" % (file["path"], file_loss))
            rejected.add(file["path"])
            _file_rejected("FILTER1 %0.3f" % file_loss, file["path"])
            status_dict["rejected"] += 1
        else:
            _file_accepted("LOSS %0.3f" % file_loss, file["path"])
            status_dict["accepted"] += 1
            all_losses.append(file_loss)
            status_dict['avg_loss'] = sum(all_losses) / len(all_losses)

        iter_times.append(time.time() - t0_iter)
        eta = (len(train_files) - iter_n) * (sum(iter_times) / len(iter_times))
        status_dict["eta_minutes"] = int(round(eta / 60))
        status_dict["worked_steps"] = iter_n
        status_dict["worked_minutes"] = int((time.time() - t0) / 60)

    traces.log("calculated frames %i " % len(train_files))
    traces.log("avg loss %0.4f" % status_dict['avg_loss'])

    return rejected


def pre_filtering(status_dict):
    finetune_cfg = get_finetune_config(logger=traces.log)

    fcfg = {**finetune_filtering_defaults}
    if os.path.exists(env.CONFIG_HOW_TO_FILTER):
        traces.log("Reading %s" % env.CONFIG_HOW_TO_FILTER)
        fcfg.update(**json.load(open(env.CONFIG_HOW_TO_FILTER)))

    has_train_files = os.path.exists(os.path.join(env.DIR_UNPACKED, unfiltered_train)) and \
                      len(list(jsonlines.open(os.path.join(env.DIR_UNPACKED, unfiltered_train))))
    if not has_train_files:
        raise RuntimeError("No train files have been provided for filtering")

    logging.info("STATUS smart filter init")
    logging.info("Train set filtering, loading model...")
    traces.log("Train set filtering, loading model...")
    t0 = time.time()
    cfg = base_config(finetune_cfg["model_name"])
    model = make_model(
        model_name=finetune_cfg["model_name"],
        weights_path=cfg['model_info']['weight_path'],
        repo_id=cfg['model_info']['repo_id'],
        backend=cfg['model_info']['backend'],
        freeze_exceptions=cfg['model_info']['freeze_exceptions'],
        lora_target_modules=cfg['model_info']['lora']['lora_target_modules'],
        lora_r=cfg['model_info']['lora']['lora_r'],
        lora_alpha=cfg['model_info']['lora']['lora_alpha'],
        lora_dropout=0,
        lora_init_scale=1e-5,
        dtype=th.bfloat16 if 'bf16' in cfg and cfg['bf16']['enabled'] else th.float16,
        init_device="cuda",
        device="cuda",
    )
    t1 = time.time()
    logging.info("/model load %0.1fms" % ((t1 - t0) * 1000))
    model.train()

    if fcfg["debug"]:
        logging.info("1 gpumem_p0 %0.2fG" % (th.cuda.max_memory_allocated() / 1e9))
        summary(model, depth=4, col_names=['num_params', 'params_percent', 'trainable'])

    dataopts = DatasetOpts("n_ctx=%d,pack_at_most=1,quit_on_epoch=1,seed=42" % (cfg['model_info']['ctx_size'] + 1))
    dataopts.set_encoding(model.encoding)
    train_files = list(jsonlines.open(unfiltered_train))
    train_files = train_files[:fcfg["limit_train_files"]]
    loss_function = partial(
        masked_loss, average_elements=cfg['model_info']['loss_average_elements'], enc=model.encoding
    )

    test_files = list(jsonlines.open(unfiltered_test))
    if len(test_files) > fcfg["limit_test_files"]:
        traces.log(f"Manually selected test set contains {len(test_files)} files, "
                   f"more than allowed {fcfg['limit_test_files']}.\n"
                   f"It could heavily slow down the training process")

    text = "FILTER explanation: initial loss too big calculated on a single file, threshold is %0.3f. " \
           "Likely means the file doesn't contain code." % fcfg["filter_loss_threshold"]
    traces.log(textwrap.fill(text, width=100))

    filtered = loss_based_filter(
        train_files, model, loss_function, dataopts, fcfg=fcfg, status_dict=status_dict,
        cfg=cfg
    )

    test_filenames = set()
    if len(test_files) == 0:
        test_files_count = min(fcfg["limit_test_files"], len(train_files) // 2)
        if test_files_count == 0:
            traces.log("Warning: It is too little files to choose a test set from. "
                       "It's strongly recommended to choose a test set manually to be able to prevent overfitting")
        else:
            test_files = random.choices(train_files, k=fcfg["limit_test_files"])
            test_filenames.update([p['path'] for p in test_files])

    with open(filtered_train, "w") as f:
        for fdict in train_files:
            p = fdict["path"]
            rejected_by_filters = p in filtered
            included_in_test_set = p in test_filenames
            if rejected_by_filters or included_in_test_set:
                continue
            f.write(json.dumps(fdict) + "\n")

    traces.log("-" * 40 + "TEST SET" + "-" * 40)
    with open(filtered_test, "w") as f:
        for fdict in test_files:
            traces.log("test set file: %s" % (fdict["path"]))
            f.write(json.dumps(fdict) + "\n")


def needs_any_work():
    try:
        has_updates = [os.path.getmtime(unfiltered_train) > os.path.getmtime(filtered_train),
                       os.path.getmtime(unfiltered_test) > os.path.getmtime(filtered_test)]
        if os.path.exists(env.CONFIG_HOW_TO_FILTER):
            has_updates.append(os.path.getmtime(env.CONFIG_HOW_TO_FILTER) > os.path.getmtime(filtered_train))
        if os.path.exists(env.CONFIG_HOW_TO_FILETYPES):
            has_updates.append(os.path.getmtime(env.CONFIG_HOW_TO_FILETYPES) > os.path.getmtime(filtered_train))
    except OSError:
        return True
    return any(has_updates)


def main(status_dict):
    if not needs_any_work():
        _update_and_dump_status(status_dict, "finished")
        logging.info("Train set filtering: nothing changed since last time, quit")
        return

    status_dict = _update_and_dump_status(status_dict, "starting")
    with open(env.LOG_FILES_ACCEPTED_FTF, "w") as f:
        f.write("")
    with open(env.LOG_FILES_REJECTED_FTF, "w") as f:
        f.write("")
    try:
        pre_filtering(status_dict)
        _update_and_dump_status(status_dict, "finished")
    except BaseException as e:  # BaseException includes KeyboardInterrupt
        if traces.context():
            logging.error("FAILED finetune filter at %s" % traces.context().path)
        if "error" not in status_dict:  # if there is, a more detailed error is already in place
            t = str(e) or str(type(e))
            status_dict["error"] = t
            logging.error(t)
            _update_and_dump_status(status_dict, "failed")
        if not isinstance(e, ValueError):  # don't print stack for ValueError which is used for mundane data problems
            raise e


if __name__ == "__main__":
    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    status_dict = get_finetune_filter_stats()

    def catch_sigusr1(signum, frame):
        status_dict["error"] = "interrupted"
        _update_and_dump_status(status_dict, "interrupted")
        sys.exit(1)

    signal.signal(signal.SIGUSR1, catch_sigusr1)
    main(status_dict)
