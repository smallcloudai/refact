import copy
import json
import logging
import math
import os
import signal
import textwrap
import time
from typing import Dict, Any

import torch

import self_hosting_machinery.finetune.utils.traces as traces
from self_hosting_machinery import env
from self_hosting_machinery.finetune.configuration.finetune_config import base_config
from self_hosting_machinery.finetune.scripts.process_uploaded_files import make_matcher
from self_hosting_machinery.finetune.scripts.script_aux.dataset import create_finetune_filter_dataloader, to_cuda
from self_hosting_machinery.finetune.scripts.script_aux.file_sets_context import FileSetsContext
from self_hosting_machinery.finetune.scripts.script_aux.file_status_context import FilesStatusContext
from self_hosting_machinery.finetune.scripts.script_aux.finetune_filter_status_tracker import \
    FinetuneFilterStatusTracker
from self_hosting_machinery.finetune.scripts.script_aux.model import ModelContext
from self_hosting_machinery.finetune.utils.finetune_utils import (get_finetune_config, get_finetune_filter_config)


def _log_everywhere(message):
    logging.info(message)
    traces.log(message)


def force_include_exclude_filter(
        files_status: FilesStatusContext
):
    fcfg = {
        "filetypes_finetune": {},
        "filetypes_db": {}
    }
    if os.path.exists(env.CONFIG_HOW_TO_FILETYPES):
        _log_everywhere("Reading %s" % env.CONFIG_HOW_TO_FILETYPES)
        with open(env.CONFIG_HOW_TO_FILETYPES, "r") as f:
            fcfg.update(**json.load(f))

    is_force_included, _ = make_matcher(fcfg.get('force_include', ''))
    is_force_excluded, _ = make_matcher(fcfg.get('force_exclude', ''))

    for file in files_status.no_status_train_files():
        if is_force_included(file['path']):
            files_status.accept_file(file, reason="FORCE_INCLUDED")
        elif is_force_excluded(file['path']):
            files_status.reject_file(file, reason="FORCE_REJECTED")


@torch.inference_mode()
def loss_based_filter(
        model_context: ModelContext,
        files_status_context: FilesStatusContext,
        status_tracker: FinetuneFilterStatusTracker,
        *,
        filter_loss_threshold
):
    def _get_file_loss(file) -> float:
        file_losses = []
        ds = create_finetune_filter_dataloader(
            file=file,
            dataset_options=f"n_ctx={model_context.finetune_cfg['model_info']['ctx_size'] + 1},"
                            "quit_on_epoch=1,pack_single=1,pack_complete=0",
            encoding=model_context.encoding
        )
        for data in map(to_cuda, ds):
            logits = model_context.forward(input=data['input'])
            loss = model_context.loss(
                logits=logits.to(torch.float32),
                labels=data['labels'],
                mask=data['mask'],
            ).item()
            if not (math.isnan(loss) or math.isinf(loss)):
                file_losses.append(loss)

        if len(file_losses) == 0:
            raise Exception("small file")

        return sum(file_losses) / len(file_losses)

    model_context.eval()
    all_losses = []
    train_files = files_status_context.no_status_train_files()
    with status_tracker(total_steps=len(train_files)) as stats_tracker:
        for file in train_files:
            try:
                file_loss = _get_file_loss(file)
            except Exception as e:
                files_status_context.reject_file(file, reason=str(e))
                continue

            if file_loss > filter_loss_threshold:
                files_status_context.reject_file(file, reason=f"loss {file_loss:.3f}")
            else:
                files_status_context.accept_file(file, reason=f"loss {file_loss:.3f}")
                all_losses.append(file_loss)

            stats_tracker.step()
    status_tracker.add_stats(avg_loss=sum(all_losses) / (len(all_losses) + 0.001))


def finetune_filter(
        status_tracker: FinetuneFilterStatusTracker,
        dataset_context: FileSetsContext,
        finetune_cfg: Dict[str, Any],
        finetune_filter_cfg: Dict[str, Any],
):
    _log_everywhere("Loading files statuses...")
    file_status_context = FilesStatusContext(
        train_files=dataset_context.train_files,
        test_files=dataset_context.test_files,
        status_tracker=status_tracker
    )

    _log_everywhere("Loading model...")
    finetune_cfg['model_info']['lora']['lora_dropout'] = 0.0
    finetune_cfg['model_info']['lora']['lora_init_scale'] = 1e-5
    finetune_cfg['model_info']['loss_average_elements'] = 1
    model_context = ModelContext(
        finetune_cfg=finetune_cfg,
    )

    _log_everywhere("Running force include/exclude filter...")
    force_include_exclude_filter(
        files_status=file_status_context
    )
    _log_everywhere("Running perplexity based filter...")
    loss_based_filter(
        model_context=model_context,
        files_status_context=file_status_context,
        status_tracker=status_tracker,
        filter_loss_threshold=finetune_filter_cfg['filter_loss_threshold']
    )

    _log_everywhere("Dumping filtered results...")
    dataset_context.dump_filtered(
        files=file_status_context.accepted_train_files(),
    )


def main(models_db: Dict[str, Any]):
    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneFilterStatusTracker()

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    _log_everywhere("Loading finetune configs...")
    finetune_filter_cfg = get_finetune_filter_config(logger=traces.log)
    model_name = get_finetune_config(models_db, logger=traces.log)["model_name"]
    finetune_cfg = copy.deepcopy(base_config(model_name, models_db))

    _log_everywhere("Loading file sets context...")
    file_sets_context = FileSetsContext(
        autoselect_test_files_num=finetune_filter_cfg.get("autoselect_test_files_num", 3)
    )
    if file_sets_context.is_up_to_date():
        logging.info("Train set filtering: nothing changed since last time, quit")
        return

    traces.log(textwrap.fill(
        f"This filter calculates perplexity for each file and filters out "
        f"files with perplexity larger than {finetune_filter_cfg['filter_loss_threshold']:.3f}.\n"
        f"Those files likely don't have meaningful content to train on", width=100
    ))
    try:
        status_tracker.update_status("starting")
        finetune_filter(
            status_tracker=status_tracker,
            dataset_context=file_sets_context,
            finetune_cfg=finetune_cfg,
            finetune_filter_cfg=finetune_filter_cfg,
        )
        status_tracker.update_status("finished")

    # finetune_sequence relies on exit code to continue or stop
    except (SystemExit, KeyboardInterrupt):
        # caught sigusr1, interrupt by watchdog or by user
        # this has to be there, even if catch_sigusr1() already called exit with 99, otherwise exit code is zero
        exit(99)
    except Exception as e:
        _log_everywhere(f"Finetune gpu filter is failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        raise e


if __name__ == "__main__":
    from known_models_db.refact_known_models import models_mini_db

    task_name = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=task_name, work_dir=env.PERMDIR)
    main(models_mini_db)
