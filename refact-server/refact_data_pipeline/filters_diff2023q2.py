import copy
import logging
import random
import traceback
from typing import Dict

import numpy as np

from code_contrast.format_2023q2 import format, packing
from code_contrast.format_2023q2.element import Format2023q2
from code_contrast.format_2023q2.from_orig_dest_message import from_odm_dict
from refact_data_pipeline import DatasetOpts
from refact_data_pipeline.datadef import PipelineNode


class Contrast2023Q2FromODM(PipelineNode):
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts):
        self.enc = dataopts.encoding
        super().__init__(dataopts)
        self.inner_filter = inner_filter
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.selftest = dataopts.get("selftest", 0)
        self.fmt: Format2023q2 = format.format_2023q2_escape(self.enc)

    def set_random_state(self, seed):
        if hasattr(self.enc, "set_random_seed"):
            self.enc.set_random_seed(seed)
        self.py_random = random.Random(seed)
        self.np_random = np.random.RandomState(seed)

    def __iter__(self):
        stats: Dict[str, int] = {
            "diffskip_5tokens": 0,
            "diffskip_toolong": 0,
            "diffskip_onlyadd": 0,
            "diffskip_failed": 0,
            "diffskip_minsize_warn": 0,
            "diffskip_noedit": 0,
        }
        for odm in self.inner_filter:
            source_files_empty_cnt = len([1 for txt in odm["orig"].values() if txt == ""])
            if source_files_empty_cnt == len(odm["orig"]):
                stats["diffskip_onlyadd"] += 1
                continue
            make_no_changes = self.py_random.random() < 0.05
            if make_no_changes:
                odm["orig"] = copy.deepcopy(odm["dest"])
            try:
                if self.selftest:
                    from code_contrast.format_2023q2 import test_2023q2
                    test_2023q2.self_test(
                        self.fmt,
                        odm,
                        verbose=False,
                        limit_ctx_n=self.n_ctx,
                        limit_aux_n=0,
                    )
                pack: packing.Packer
                pack, msg_plan_n = from_odm_dict(
                    self.fmt,
                    odm,
                    for_training=True,
                    exact_cx_lines0=-1,
                    exact_cx_lines1=-1,
                    want_cursor_token=True,
                    random_state=self.np_random
                )
                if len(pack.plan) - 1 == msg_plan_n and not make_no_changes:
                    stats["diffskip_noedit"] += 1
                    continue
                pack.pack_context(
                    start_from_plan_n=0,
                    mask_from_plan_n=msg_plan_n,
                    limit_ctx_n=self.n_ctx,
                    limit_aux_n=0,
                    add_eot=True,
                    for_training=True
                )
                # edits_made = len(pack.plan) - 1 - msg_plan_n
                # print("edits: %i" % edits_made)
                # if edits_made == 1:
                # Interesting, cursor might appear
                # print(hlprint(self.enc, pack.r, pack.m))

            except Exception as e:
                msg = "{\n"
                for key, val in odm.items():
                    if key not in ["orig", "dest", "commitmsg"]:
                        continue
                    msg += f"    {repr(key)}: {repr(val)},\n"
                msg += "}"
                logging.error(msg)
                logging.error(traceback.format_exc())
                stats["diffskip_failed"] += 1
                # Fixing bugs: copy odm from console to test_2023q2.py, run it to reproduce
                continue
            with open("pack_size2.csv", "at") as f:
                f.write(f"{len(pack.r)}\n")
            if len(pack.r) > self.n_ctx + 100:
                stats["diffskip_toolong"] += 1
                continue
            unmasked = sum(pack.m[:self.n_ctx])
            if unmasked < 5:
                stats["diffskip_5tokens"] += 1
                continue
            if pack.cx.minimal_context_too_big_warning:
                stats["diffskip_minsize_warn"] += 1  # don't skip, continue
            first = [1] + [0] * (len(pack.r) - 1)
            assert len(pack.r) == len(first)
            assert len(pack.r) == len(pack.m)
            emit = {
                "tokens": pack.r,
                "mask": pack.m,
                "first": first,
                "stats": {**odm["stats"], **stats}
            }
            yield emit
