import click
import copy
import json
import logging
import math
import os
import signal
import textwrap
import traceback
import time
from typing import Dict, Any, Tuple

import torch

from refact_utils.scripts import env
from refact_utils.finetune.utils import (get_finetune_config, get_finetune_filter_config)
import self_hosting_machinery.finetune.utils.traces as traces
from self_hosting_machinery.finetune.configuration.finetune_config import base_config
from self_hosting_machinery.finetune.scripts.auxiliary.dataset import (
    create_finetune_filter_dataloader, to_cuda, setup_encoding
)
from self_hosting_machinery.finetune.scripts.auxiliary.file_sets_context import FileSetsContext
from self_hosting_machinery.finetune.scripts.auxiliary.file_status_context import FilesStatusContext
from self_hosting_machinery.finetune.scripts.auxiliary.finetune_filter_status_tracker import FinetuneFilterStatusTracker
from self_hosting_machinery.finetune.scripts.auxiliary.model import ModelContext
from self_hosting_machinery.finetune.scripts.process_uploaded_files import make_matcher


class InvalidLossValueException(Exception):
    pass


def _log_everywhere(message):
    logging.info(message)
    traces.log(message)


def force_include_exclude_filter(
    pname: str,
    files_status: FilesStatusContext
):
    fcfg = {
        "filetypes_finetune": {},
        "filetypes_db": {}
    }
    if os.path.exists(env.PP_CONFIG_HOW_TO_FILETYPES(pname)):
        _log_everywhere("Reading %s" % env.PP_CONFIG_HOW_TO_FILETYPES(pname))
        with open(env.PP_CONFIG_HOW_TO_FILETYPES(pname), "r") as f:
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
        pname: str,
        finetune_cfg: Dict[str, Any],
        dataset_context: FileSetsContext,
        files_status_context: FilesStatusContext,
        status_tracker: FinetuneFilterStatusTracker,
        *,
        filter_loss_threshold
):
    def _get_file_loss(model_context, file) -> Tuple[ModelContext, float]:
        file_losses = []
        ds = create_finetune_filter_dataloader(
            pname=pname,
            file=file,
            dataset_options=f"n_ctx={finetune_cfg['model_info']['ctx_size'] + 1},"
                            "quit_on_epoch=1,pack_single=1,pack_complete=0",
            encoding=encoding
        )
        for data in map(to_cuda, ds):
            content = encoding.decode(data['input'][0])
            maybe_loss = dataset_context.get_loss_by_content(
                model_name=finetune_cfg["model_name"],
                content=content
            )
            if maybe_loss is not None:
                loss = maybe_loss
            else:
                if model_context is None:
                    model_context = ModelContext(finetune_cfg=finetune_cfg)
                    model_context.eval()

                logits = model_context.forward(input=data['input'])
                loss = model_context.loss(
                    logits=logits.to(torch.float32),
                    labels=data['labels'],
                    mask=data['mask'],
                ).item()
                dataset_context.add_content_loss_pair(
                    model_name=model_context.model_name,
                    content=content,
                    loss=loss
                )
            if not (math.isnan(loss) or math.isinf(loss)):
                file_losses.append(loss)

        if len(file_losses) == 0:
            raise InvalidLossValueException("small file")

        return model_context, sum(file_losses) / len(file_losses)

    encoding = setup_encoding(
        model_name=finetune_cfg["model_name"],
        weights_path=finetune_cfg['model_info']['weight_path'],
        repo_id=finetune_cfg['model_info']['repo_id']
    )
    model_context = None
    all_losses = []
    train_files = files_status_context.no_status_train_files()
    with status_tracker(total_steps=len(train_files)) as stats_tracker:
        for file in train_files:
            try:
                model_context, file_loss = _get_file_loss(model_context, file)
            except InvalidLossValueException as e:
                files_status_context.reject_file(file, reason=str(e))
                continue
            except Exception as e:
                import traceback
                traces.log(traceback.format_exc())
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
        pname,
        status_tracker: FinetuneFilterStatusTracker,
        dataset_context: FileSetsContext,
        finetune_cfg: Dict[str, Any],
        finetune_filter_cfg: Dict[str, Any],
):
    _log_everywhere("Loading files statuses...")
    file_status_context = FilesStatusContext(
        pname=pname,
        train_files=dataset_context.train_files,
        test_files=dataset_context.test_files,
        status_tracker=status_tracker
    )

    _log_everywhere("Loading model...")
    finetune_cfg['model_info']['lora']['lora_dropout'] = 0.0
    finetune_cfg['model_info']['loss_average_elements'] = 1

    _log_everywhere("Running force include/exclude filter...")
    force_include_exclude_filter(
        pname,
        files_status=file_status_context
    )
    _log_everywhere("Running perplexity based filter...")
    loss_based_filter(
        pname,
        finetune_cfg=finetune_cfg,
        dataset_context=dataset_context,
        files_status_context=file_status_context,
        status_tracker=status_tracker,
        filter_loss_threshold=finetune_filter_cfg['filter_loss_threshold']
    )

    _log_everywhere("Dumping filtered results...")
    dataset_context.dump_filtered(
        files=file_status_context.accepted_train_files(),
    )


@click.command()
@click.option('--pname', default='')
def main(pname):
    _log_everywhere("Loading status tracker...")
    status_tracker = FinetuneFilterStatusTracker(pname)
    from known_models_db.refact_known_models import models_mini_db
    models_db: Dict[str, Any] = models_mini_db

    def catch_sigusr1(signum, frame):
        _log_everywhere("catched SIGUSR1, interrupted")
        status_tracker.update_status("interrupted", error_message="catched SIGUSR1, interrupted")
        exit(99)

    signal.signal(signal.SIGUSR1, catch_sigusr1)

    _log_everywhere("Loading finetune configs...")
    finetune_filter_cfg = get_finetune_filter_config(logger=traces.log)
    model_name = get_finetune_config(models_db, logger=traces.log)["model_name"]
    finetune_cfg = copy.deepcopy(base_config(model_name, models_db))

    try:
        _log_everywhere("Loading file sets context...")
        file_sets_context = FileSetsContext(
            pname=pname,
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

        status_tracker.update_status("starting")
        finetune_filter(
            pname=pname,
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
        traces.log(traceback.format_exc())
        _log_everywhere(f"Finetune gpu filter is failed\nException: {e}")
        status_tracker.update_status("failed", error_message=str(e) or str(type(e)))
        raise e


if __name__ == "__main__":
    task_name = os.environ.get("LORA_LOGDIR", "") or time.strftime("lora-%Y%m%d-%H%M%S")
    traces.configure(task_dir="loras", task_name=task_name, work_dir=env.PERMDIR)
    main()
