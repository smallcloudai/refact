import functools

from code_contrast.format_2022q3 import contrast
from refact_encoding import RefactEncoding

from refact_data_pipeline import DatasetOpts

from typing import List


def _diff_valid_decode_result(enc, odm, tokens_with_completion: List[int]):
    diff = contrast.ContrastDiff(enc)
    dest_tokens = dict()
    edit_stats = dict()
    try:
        us = diff.untokenize(tokens_with_completion, full_orig_tokens=odm["orig_tokens"])
        dest_tokens = diff.apply_edits_return_dest(us)
        edit_stats = us.stats
    except Exception:
        pass
    ret = dict()
    ret["errors"] = diff.errors
    ret["edit_stats"] = edit_stats
    ret["dest_tokens"] = dest_tokens
    ret["tokens_with_completion"] = [int(t) for t in tokens_with_completion]
    ret["code"] = {
        fn: enc.decode(tokens)
        for fn, tokens in dest_tokens.items()
    }
    return ret


class DiffValidInstructPrompt:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc: RefactEncoding = dataopts.encoding

    def __iter__(self):
        for ex in self.inner_filter:
            odm = {
                "orig": {fn: text for fn, text in ex["orig"].items()},
                "dest": {fn: text for fn, text in ex["dest"].items()},
                "commitmsg": ex["commitmsg"],
                "stats": ex["stats"],
            }
            diff = contrast.ContrastDiff(self.enc)
            diff.from_odm_dict(
                odm,
                n_ctx=1024,
            )
            diff.write_edits()
            ex["prompt_tokens"] = diff.r[:diff.offset_first_postoken]
            ex["completion_tokens"] = diff.r[diff.offset_first_postoken:]
            odm["orig_tokens"] = diff.orig_tokens
            ex["decode_result_fn"] = functools.partial(_diff_valid_decode_result, self.enc, odm)
            if 0:
                import termcolor
                import difflib
                out = ex["decode_result_fn"](diff.r)
                for fn in out["code"].keys():
                    print(termcolor.colored(out["code"][fn], "red"))
                    print(termcolor.colored(ex["dest"][fn], "blue"))
                    print("".join(difflib.unified_diff(
                        out["code"][fn].splitlines(keepends=True),
                        ex["dest"][fn].splitlines(keepends=True),
                        fromfile="code",
                        tofile="dest",
                    )))
                    assert out["code"][fn] == ex["dest"][fn]
            yield ex

