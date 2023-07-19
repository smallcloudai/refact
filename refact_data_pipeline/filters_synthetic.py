import random
import traceback
import re

import numpy as np

from refact_data_pipeline import DatasetOpts
from code_contrast.format_2022q3 import contrast
from refact_encoding import RefactEncoding

from typing import Dict


class InfillDiff:
    def __init__(self,
                 inner_filter,
                 dataopts: DatasetOpts,
                 ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc: RefactEncoding = dataopts.encoding
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.contrast_unmask_orig = 0  #dataopts.get("contrast_unmask_orig", 0)
        self.seed = dataopts.get("seed", 0)
        self.py_random = random.Random(self.seed if self.seed else None)
        self.np_random = np.random.RandomState(self.seed if self.seed else None)

    def __iter__(self):
        stats: Dict[str, int] = {
            "infill_filt_rowcnt": 0,
            "infill_filt_5tokens": 0,
            "infill_not_one_edit": 0,
            "infill_out": 0,
        }
        for d in self.inner_filter:
            # keys ['code', 'repo_name', 'path', 'language', 'license', 'size']
            fn = d["path"]
            code = d["code"]
            if d["size"] > 30_000:
               continue
            tmp = self.enc.encode(code)
            div_into = int(len(tmp) / (self.n_ctx*15/16 - 128)) + 1
            div_chars = int(len(code) / div_into) + 1
            for i in range(div_into):
                div_code = code[i*div_chars : (i+1)*div_chars]
                code_rows = div_code.split("\n")
                rows_cnt = len(code_rows)
                if rows_cnt < 11:
                    stats["infill_filt_rowcnt"] += 1
                    continue
                while 1:
                    # cut up to 20 lines of code
                    cutlines = self.py_random.randint(0, 10)
                    cut1line = self.py_random.randint(0, rows_cnt - 1 - cutlines)
                    first_line = code_rows[cut1line]
                    if cutlines == 0:
                        if len(first_line) < 4:
                            continue
                        cut1pos = self.py_random.randint(0, len(first_line) - 2)
                    else:
                        cut1pos = self.py_random.randint(0, len(first_line))
                    if self.py_random.randint(0, 1):
                        cut1pos = 0
                    part1 = ""
                    for x in code_rows[:cut1line]:
                        part1 += x + "\n"
                    part1 += code_rows[cut1line][:cut1pos]
                    if cutlines == 0:
                        cut2pos = self.py_random.randint(cut1pos + 1, len(first_line))
                        part2 = first_line[cut2pos:]
                        if len(code_rows[cut1line + 1:]):
                            part2 += "\n" + "\n".join(code_rows[cut1line + 1:])
                    else:
                        part2 = "\n".join(code_rows[cut1line + cutlines:])
                    assert div_code[:len(part1)] == part1
                    if len(part2) > 0:
                        if div_code[-len(part2):] != part2:
                            stats["infill_not_one_edit"] += 1
                        assert div_code[-len(part2):] == part2
                    removed = div_code[len(part1):-len(part2)]
                    if re.fullmatch(r"\s+", removed) and self.py_random.randint(0, 5) > 0:
                        continue
                    # newline_cnt = removed.count("\n")
                    # traces.log("infill", f"removed {len(removed)} chars from {len(div_code)}, has newlines {newline_cnt}")
                    if len(removed) > 0:
                        break
                try:
                    odm = {
                        "orig": {fn: part1 + self.enc.decode([self.enc.INFILL]) + part2},
                        "dest": {fn: div_code},
                        "commitmsg": "Infill",
                        "stats": {"infill_part": i, **d["stats"]},
                    }
                    diff = contrast.ContrastDiff(self.enc)
                    diff.from_odm_dict(
                        odm,
                        n_ctx=self.n_ctx,
                        contrast_unmask_orig=self.contrast_unmask_orig,
                        random_shrink=False,
                        np_random=self.np_random,
                    )
                    if len(diff.edits) != 1:  # mostly normal, diff sometimes separates changes in two parts
                        continue
                    assert len(diff.edits) == 1, len(diff.edits)
                    assert self.enc.INFILL in diff.r
                    # from code_contrast.print_utils import hlprint
                    # print(hlprint(self.enc, diff.r))
                    # print(diff.dump_edits())
                    # Don't predict infill
                    infill_position = diff.r.index(self.enc.INFILL)
                    assert infill_position != -1
                    diff.m[infill_position] = 0
                    diff.write_edits()

                except Exception as e:
                    print(str(odm))
                    print(traceback.format_exc())
                    continue
                edits_within_context = self.n_ctx - diff.offset_edits
                if edits_within_context < 5:
                    stats["infill_filt_5tokens"] += 1
                    continue
                stats["infill_out"] += 1
                first = [1] + [0]*(len(diff.r) - 1)
                edit_classes = [0]*len(diff.r)
                hlpoint = [0]*len(diff.r)
                yield {
                    "tokens": diff.r,
                    "mask": diff.m,
                    "first": first,
                    "diffhlpoint": hlpoint,
                    "diffedits": edit_classes,
                    "stats": {**odm["stats"], **stats},
                }
            #     if div_into > 1:
            #         import termcolor
            #         result_len = len(diff.r)  # diff.tokens_without_shortening
            #         msg = "%04i" % (result_len,)
            #         if result_len > self.n_ctx:
            #             print(termcolor.colored(msg, "red"))
            #         else:
            #             print(msg)
            # print()

