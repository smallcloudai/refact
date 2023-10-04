import logging
import random
import traceback
import copy

from refact_encoding import RefactEncoding
from refact_data_pipeline import DatasetOpts
from code_contrast.format_2022q3 import contrast

from typing import Dict


class ContrastFromODM:
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts):
        self.inner_filter = inner_filter
        self.enc: RefactEncoding = dataopts.encoding
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.selftest = dataopts.get("selftest", 0)

    def __iter__(self):
        stats: Dict[str, int] = {
            "diffskip_5tokens": 0,
            "diffskip_toobig": 0,
            "diffskip_onlyadd": 0,
            "diffskip_selftest": 0,
            "diffskip_failed": 0,
            "diffskip_noedit": 0,
        }
        for odm in self.inner_filter:
            source_files_empty_cnt = len([1 for txt in odm["orig"].values() if txt == ""])
            if source_files_empty_cnt == len(odm["orig"]):
                stats["diffskip_onlyadd"] += 1
                continue
            make_no_changes = random.random() < 0.05
            if make_no_changes:
                odm["orig"] = copy.deepcopy(odm["dest"])
            if self.selftest:
                try:
                    contrast.self_test(self.enc, odm, verbose=False, n_ctx=self.n_ctx)
                except contrast.TooBig:
                    stats["diffskip_toobig"] += 1
                    continue
                except Exception as e:
                    print(str(odm))
                    print(traceback.format_exc())
                    stats["diffskip_selftest"] += 1
                    continue
            diff = contrast.ContrastDiff(self.enc)
            try:
                diff.from_odm_dict(
                    odm,
                    n_ctx=self.n_ctx,
                )
                if len(diff.edits) == 0 and not make_no_changes:
                    stats["diffskip_noedit"] += 1
                    continue
                diff.write_edits()
                edit_classes = diff.edit_class_vector()
                # edit classes:
                # 0: no training
                # 1: no edit
                # 2: edit
                # 3: continue edit
                hlpoint = [0]*len(diff.r)
                # hlpoint:
                # 0 0 0 0 0 0 0 1 0 0
                #               ^ first position token decision -- the model decides where to make changes
                hlpoint[diff.offset_first_postoken] = 1
                assert len(diff.r) == len(edit_classes)
            except contrast.TooBig:
                stats["diffskip_toobig"] += 1
                continue
            except Exception as e:
                logging.error(str(odm))
                logging.error(traceback.format_exc())
                stats["diffskip_failed"] += 1
                continue
            edits_within_context = self.n_ctx - diff.offset_edits
            # if 1:
            #     import termcolor
            #     print(termcolor.colored(str(len(diff.r)), "green" if len(diff.r) < self.n_ctx else "red"), edits_within_context)
            if edits_within_context < 5:
                stats["diffskip_5tokens"] += 1
                continue
            first = [1] + [0]*(len(diff.r) - 1)
            assert len(diff.r) == len(first)
            assert len(diff.r) == len(diff.m)
            assert len(diff.r) == len(edit_classes)
            assert len(diff.r) == len(hlpoint)
            emit = {
                "tokens": diff.r,
                "mask": diff.m,
                "first": first,
                "diffhlpoint": hlpoint,
                "diffedits": edit_classes,
                # "diffshifts": edit_shifts,
                "stats": {**odm["stats"], **stats}
            }
            yield emit

