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
from refact_data_pipeline.finetune import finetune_filtering_defaults
from refact_data_pipeline.finetune.finetune_config import base_config
from refact_data_pipeline.finetune.model_handling import make_model, masked_loss
from refact_data_pipeline.finetune.finetune_train import save_status_json
from refact_data_pipeline.finetune.process_uploaded_files import make_matcher
from self_hosting_machinery import env

from typing import List


unfiltered_train = os.path.join(env.DIR_UNPACKED, "train_set.jsonl")
unfiltered_test = os.path.join(env.DIR_UNPACKED, "test_set.jsonl")

filtered_train = os.path.join(env.DIR_UNPACKED, "train_set_filtered.jsonl")
filtered_test = os.path.join(env.DIR_UNPACKED, "test_set_filtered.jsonl")

status_dict = {
    "started_ts": time.time(),
    "total_steps": 0,
    "worked_steps": 0,
    "worked_minutes": 0,
    "eta_minutes": 0,
    "status": "starting",
    "accepted": 0,
    "rejected": 0,
    "avg_loss": 0.0
}


def _save_stats(status_string):
    save_status_json(status_dict, status_string)
    with open(env.CONFIG_FINETUNE_FILTER_STATS + ".tmp", "w") as f:
        json.dump(status_dict, f, indent=4)
    os.rename(env.CONFIG_FINETUNE_FILTER_STATS + ".tmp", env.CONFIG_FINETUNE_FILTER_STATS)


def _try_load_stats():
    global status_dict
    if not os.path.exists(env.CONFIG_FINETUNE_FILTER_STATS):
        return
    with open(env.CONFIG_FINETUNE_FILTER_STATS, "r") as f:
        status_dict = json.load(f)


def _file_accepted(reason, path):
    with open(env.LOG_FILES_ACCEPTED_FTF, "a") as f:
        f.write("%s %s\n" % (reason, path))


def _file_rejected(reason, path):
    with open(env.LOG_FILES_REJECTED_FTF, "a") as f:
        f.write("%s %s\n" % (reason, path))


def catch_sigusr1(signum, frame):
    status_dict["error"] = "interrupted"
    _save_stats("interrupted")
    sys.exit(1)


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
        fcfg
):
    t0 = time.time()
    iter_times = []
    model.eval()
    batch_iter_fn = partial(BatchIterator, dataopts=dict(batch_size=1, drop_last=False))
    all_losses, rejected = [], set()
    logging.info("STATUS filtering")
    status_dict['total_steps'] = len(train_files)
    is_force_included, is_force_excluded = get_force_included_excluded_matchers()
    for iter_n, file in enumerate(train_files):
        t0_iter = time.time()
        _save_stats("filtering")
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
            logits = model.lm_forward(model(batch['input'].contiguous(), attention_mask=None)[0])
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

    return rejected, None


def twostage_filter(
        train_files,
        model,
        loss_function,
        dataopts,
        *,
        fcfg
):
    t0 = time.time()
    grad31_name = ""
    grad31_size = []
    for n, p in model.named_parameters():
        if n == "blocks.31.sa.qkv.lora_B.weight":
            grad31_name = n
            grad31_size = p.numel()  # [7680, 16]
    assert grad31_size
    all_documents_grad31 = th.zeros([grad31_size], dtype=th.float32, device="cuda")

    def dig_out_grad31():
        for n, p in model.named_parameters():
            # if p.grad is not None and "lora_B" in n and "qkv.lora_B" in n:
            #     traces.log("%s %s mean=%0.6f std=%0.6f" % (n, p.shape, p.grad.mean(), p.grad.std()))
            if n == grad31_name:
                if not th.isnan(p.grad).any():
                    return p.grad.flatten()
                else:
                    return None

    scale = 65536
    most_typical_file_fdict = None
    most_typical_file_cos = -1
    rejected = set()
    status_dict['total_steps'] = len(train_files)
    is_force_included, is_force_excluded = get_force_included_excluded_matchers()
    for Pass in [1, 2]:
        loss_list = []
        logging.info("STATUS filtering %d" % Pass)
        for iter_n, fdict in enumerate(train_files):
            t0_iter = time.time()
            _save_stats("filtering %d" % Pass)
            test_ds = finetune_datasource.local_plain([fdict], dataopts)
            test_batch_iter_fn = partial(BatchIterator, dataopts=dict(batch_size=1, drop_last=False))
            if is_force_included(fdict['path']):
                _file_accepted("INCLUDED_BY_MASK", fdict["path"])
                status_dict["accepted"] += 1
                continue
            elif is_force_excluded(fdict['path']):
                traces.log("REJECTED FILTER %-100s EXCLUDED_BY_MASK" % fdict["path"])
                rejected.add(fdict["path"])
                _file_rejected("FILTER1 EXCLUDED_BY_MASK", fdict["path"])
                status_dict["rejected"] += 1
                continue
            for batch, _stats in test_batch_iter_fn(test_ds):
                input = batch['input'][:].contiguous()
                while 1:
                    model.zero_grad()
                    if fcfg["low_gpu_mem_mode"]:
                        logits = model.forward_train_cp(input)
                    else:
                        logits = model.lm_forward(model(input, attention_mask=None)[0])
                    loss = loss_function(
                        logits=logits,
                        labels=batch['labels'],
                        mask=batch['mask'],
                    )
                    floss = float(loss.item())
                    loss *= float(scale)
                    loss.backward()
                    grad31 = dig_out_grad31()
                    if grad31 is None:
                        traces.log("scaling %d -> %d" % (scale, scale // 2))
                        scale //= 2
                        if scale <= 128:
                            traces.log("The `scale` is too low, can't complete the task :(")
                            traces.log("One potential way to fix this is to delete very unusual files, "
                                       "this last one was:\n%s" % fdict["path"])
                            sys.exit(1)
                        continue
                    break
                assert floss is not None
                loss_list.append(floss)
                status_dict['avg_loss'] = sum(loss_list) / len(loss_list)
                if Pass == 1:
                    if floss > fcfg['filter_loss_threshold']:
                        traces.log("REJECTED FILTER1 %-100s loss %0.3f" % (fdict["path"], floss))
                        rejected.add(fdict["path"])
                        _file_rejected("FILTER1 %0.3f" % (floss,), fdict["path"])
                        status_dict["rejected"] += 1
                    else:
                        all_documents_grad31 += grad31
                elif Pass == 2:
                    cos = th.nn.functional.cosine_similarity(all_documents_grad31, grad31.float(), dim=0)
                    if cos < fcfg['filter_gradcosine_threshold']:
                        traces.log("REJECTED FILTER2 %-100s cosine_similarity %+0.3f" % (fdict["path"], cos))
                        rejected.add(fdict["path"])
                        _file_rejected("FILTER2 %+0.3f" % (cos,), fdict["path"])
                        status_dict["rejected"] += 1
                    else:
                        _file_accepted("LOSS %0.3f COSINE %+0.3f" % (floss, cos), fdict["path"])
                        status_dict["accepted"] += 1
                    if cos > most_typical_file_cos:
                        most_typical_file_cos = cos
                        most_typical_file_fdict = fdict
                break
            iter_time = time.time() - t0_iter
            if iter_time_last is None:
                eta = (len(train_files) - iter_n) * iter_time
            else:
                eta = (len(train_files) - iter_n) * ((iter_time + iter_time_last) / 2)
            iter_time_last = iter_time
            status_dict["eta_minutes"] = int(round(eta / 60))
            status_dict["worked_steps"] = iter_n
            status_dict["worked_minutes"] = int((time.time() - t0) / 60)
        traces.log("calculated frames %i " % len(loss_list))
        traces.log("avg loss %0.4f" % status_dict['avg_loss'])
    return rejected, most_typical_file_fdict


def pre_filtering():
    fcfg = {**finetune_filtering_defaults.finetune_filtering_defaults}
    if os.path.exists(env.CONFIG_HOW_TO_FILTER):
        traces.log("Reading %s" % env.CONFIG_HOW_TO_FILTER)
        fcfg.update(**json.load(open(env.CONFIG_HOW_TO_FILTER)))

    logging.info("STATUS smart filter init")
    logging.info("Train set filtering, loading model...")
    traces.log("Train set filtering, loading model...")
    t0 = time.time()
    cfg = base_config(env)
    model = make_model(
        weights_path=cfg['model_info']['weight_path'],
        repo_id=cfg['model_info']['repo_id'],
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

    text = "FILTER1 explanation: initial loss too big calculated on a single file, threshold is %0.3f. " \
           "Likely means the file doesn't contain code." % fcfg["filter_loss_threshold"]
    traces.log(textwrap.fill(text, width=100))
    if fcfg["gradient_based_filter"]:
        text = "FILTER2 explanation: gradient cosine similarity is bad, calculated on a file, threshold is %0.3f. " \
               "This means the file does not pull the model in the same direction as the rest of the files." \
               % fcfg["filter_gradcosine_threshold"]
        traces.log(textwrap.fill(text, width=100))

    filter = twostage_filter if fcfg["gradient_based_filter"] else loss_based_filter
    filtered, most_typical_file_fdict = filter(
        train_files, model, loss_function, dataopts, fcfg=fcfg
    )

    test_filenames = set()
    if most_typical_file_fdict is None and len(test_files) == 0:
        traces.log("no most_typical_file_fdict was found, create the new test set")
        test_files = random.choices(train_files, k=fcfg["limit_test_files"])
        test_filenames.update([p['path'] for p in test_files])
    elif most_typical_file_fdict is not None:
        traces.log("detected \"most typical\" file: %s" % (most_typical_file_fdict["path"]))
        if len(test_files) > 0:
            most_typical_file_fdict = None
        else:
            test_filenames.add(most_typical_file_fdict["path"])

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
        if most_typical_file_fdict is not None:
            traces.log("test set is auto selected, consists of one file called: %s" % most_typical_file_fdict["path"])
            traces.log("this file is removed from the train set.")
            f.write(json.dumps(most_typical_file_fdict) + "\n")
        else:
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


def main():
    if not needs_any_work():
        _try_load_stats()
        _save_stats("finished")
        logging.info("Train set filtering: nothing changed since last time, quit")
        return

    _save_stats("starting")
    with open(env.LOG_FILES_ACCEPTED_FTF, "w") as f:
        f.write("")
    with open(env.LOG_FILES_REJECTED_FTF, "w") as f:
        f.write("")
    try:
        pre_filtering()
        _save_stats("finished")
    except BaseException as e:  # BaseException includes KeyboardInterrupt
        if traces.context():
            logging.error("FAILED finetune filter at %s" % traces.context().path)
        if "error" not in status_dict:  # if there is, a more detailed error is already in place
            t = str(e) or str(type(e))
            status_dict["error"] = t
            logging.error(t)
            _save_stats("failed")
        if not isinstance(e, ValueError):  # don't print stack for ValueError which is used for mundane data problems
            raise e


if __name__ == "__main__":
    YMD_hms = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=YMD_hms, work_dir=env.PERMDIR)
    signal.signal(signal.SIGUSR1, catch_sigusr1)
    main()
