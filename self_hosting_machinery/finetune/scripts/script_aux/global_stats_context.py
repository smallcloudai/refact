import json
import os
import time
from typing import Dict, Any, Optional

from self_hosting_machinery.finetune.utils.eta import EtaTracker
from self_hosting_machinery.finetune.utils.finetune_utils import get_finetune_filter_stat
from self_hosting_machinery.scripts import env

__all__ = ['GlobalStatsContext']


class GlobalStatsContext:
    class StatusDictTracker:
        def __init__(self, context, total_steps: int):
            self.context: GlobalStatsContext = context
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
        self._stats_dict = get_finetune_filter_stat(default=True)
        self._tracker_extra_kwargs: Dict[str, Any] = dict()

    def dump(self):
        with open(env.CONFIG_FINETUNE_FILTER_STAT + ".tmp", "w") as f:
            json.dump(self._stats_dict, f, indent=4)
        os.rename(env.CONFIG_FINETUNE_FILTER_STAT + ".tmp", env.CONFIG_FINETUNE_FILTER_STAT)

    def update_status(
            self,
            status: str,
            error_message: Optional[str] = None,
            dump: bool = True
    ):
        env.report_status("filter", status)
        self._stats_dict["filtering_status"] = status
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

    def __enter__(self) -> 'GlobalStatsContext.StatusDictTracker':
        self.add_stats(**self._tracker_extra_kwargs)
        return GlobalStatsContext.StatusDictTracker(context=self, **self._tracker_extra_kwargs)

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass

    def add_stats(self, **kwargs):
        self._stats_dict.update(kwargs)
        self.dump()
