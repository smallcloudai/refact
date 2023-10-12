import json
import os
import time
from pathlib import Path
from typing import Dict, Any, Optional

from self_hosting_machinery.finetune.utils.eta import EtaTracker
from self_hosting_machinery.finetune.utils.finetune_utils import get_finetune_filter_stat
from self_hosting_machinery.scripts import env
from self_hosting_machinery.finetune.utils import traces

__all__ = ['FinetuneStatusTracker']


def get_finetune_status() -> Dict[str, Any]:
    return {
        "started_ts": time.time(),
        "worked_steps": 0,
        "worked_minutes": 0,
        "status": "starting",
        "quality": "unknown"
    }


class FinetuneStatusTracker:
    class LoopStatusTracker:
        def __init__(self, context, total_steps: int):
            self.context: FinetuneStatusTracker = context
            self.eta_tracker = EtaTracker(total_steps)
            self.iter_n = 0
            self.initial_iter_tp = time.time()
            self.last_iter_tp = time.time()

        def step(self):
            self.eta_tracker.append(time.time() - self.last_iter_tp)
            self.context._stats_dict["eta_minutes"] = int(round(self.eta_tracker.eta() / 60))
            self.context._stats_dict["worked_steps"] = self.iter_n
            self.context._stats_dict["worked_minutes"] = int((time.time() - self.initial_iter_tp) / 60)
            self.context.dump()
            self.iter_n += 1
            self.last_iter_tp = time.time()

    def __init__(self):
        self._stats_dict = get_finetune_status()
        self._rank = os.environ.get('RANK', 0)
        self._tracker_extra_kwargs: Dict[str, Any] = dict()
        self._status_filename = Path(traces.context().path) / "status.json"

    def dump(self):
        if self._rank != 0:
            return

        traces.touch()
        if not traces.context():
            return
        with open(self._status_filename.with_suffix(".tmp"), "w") as f:
            json.dump(self._stats_dict, f, indent=4)
        os.rename(self._status_filename.with_suffix(".tmp"), self._status_filename)

    def update_status(
            self,
            status: str,
            error_message: Optional[str] = None,
            dump: bool = True
    ):
        env.report_status("ftune", status)
        self._stats_dict["status"] = status
        if error_message is not None:
            assert status in {"failed", "interrupted"}
            self._stats_dict["error"] = error_message
        if dump:
            self.dump()

    def set_accepted_num(self, num: int, dump: bool = True):
        self._stats_dict["accepted"] = num
        if dump:
            self.dump()

    def set_rejected_num(self, num: int, dump: bool = True):
        self._stats_dict["rejected"] = num
        if dump:
            self.dump()

    def __call__(self, **kwargs):
        self._tracker_extra_kwargs.clear()
        self._tracker_extra_kwargs.update(kwargs)
        return self

    def __enter__(self) -> 'FinetuneStatusTracker.LoopStatusTracker':
        self.add_stats(**self._tracker_extra_kwargs)
        return FinetuneStatusTracker.LoopStatusTracker(context=self, **self._tracker_extra_kwargs)

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass

    def add_stats(self, **kwargs):
        self._stats_dict.update(kwargs)
        self.dump()
