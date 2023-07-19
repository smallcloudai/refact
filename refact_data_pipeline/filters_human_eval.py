import functools

from code_contrast.format_2022q3 import contrast
from refact_encoding import RefactEncoding
from refact_data_pipeline import DatasetOpts

from typing import List


class HumanEvalContinuation:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc: RefactEncoding = dataopts.encoding

    def decode_result(self, prompt, tokens_with_completion: List[int]):
        txt = self.enc.decode(tokens_with_completion, cut_at_eot=True)
        assert txt.startswith(prompt)
        completion = txt[len(prompt):]
        for stop in ["\nclass", "\ndef", "\n#", "\nif", "\nprint"]:
            if stop in completion:
                i = completion.find(stop)
                assert i != -1
                completion = completion[:i]
        ret = {
            "completion": completion,
            "tokens_with_completion": [int(t) for t in tokens_with_completion],
        }
        return ret

    def __iter__(self):
        for ex in self.inner_filter:
            ex["prompt"] = ex["prompt"].strip()
            ex["completion"] = ex["canonical_solution"]
            ex["decode_result_fn"] = functools.partial(self.decode_result, ex["prompt"])
            yield ex


def _human_diff_decode_result(enc, odm, tokens_with_completion: List[int]):
    diff = contrast.ContrastDiff(enc)
    fn = next(iter(odm["orig_tokens"]))
    ret = dict()
    dest_tokens = dict()
    edit_stats = dict()
    try:
        us = diff.untokenize(tokens_with_completion, full_orig_tokens=odm["orig_tokens"])
        dest_tokens = diff.apply_edits_return_dest(us)
        edit_stats = us.stats
    except Exception:
        pass
    txt = enc.decode(dest_tokens.get(fn, []))
    if txt.startswith(odm["original_prompt"]):
        completion = txt[len(odm["original_prompt"]):]
        for stop in ["\nclass", "\ndef", "\n#", "\nif", "\nprint"]:
            if stop in completion:
                i = completion.find(stop)
                assert i != -1
                completion = completion[:i]
        txt = txt[:len(odm["original_prompt"])] + completion
    ret["errors"] = diff.errors
    ret["edit_stats"] = edit_stats
    ret["dest_tokens"] = diff.dest_tokens
    ret["completion"] = txt
    ret["tokens_with_completion"] = [int(t) for t in tokens_with_completion]
    # "completion_tokens" unchanged, will contain canonical solution
    return ret


class HumanEvalInfillPrompt:
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
            # dict_keys(['task_id', 'prompt', 'entry_point', 'canonical_solution', 'test', 'stats'])
            fn = ex["entry_point"] + ".py"
            part1 = ex["prompt"] + "    "
            part2 = "\n\n"
            dest = ex["prompt"] + ex["canonical_solution"]
            odm = {
                "orig": {fn: part1 + self.enc.decode([self.enc.INFILL]) + part2},
                "dest": {fn: dest},
                "commitmsg": "Infill",
                "original_prompt": ex["prompt"].rstrip(),
                "stats": ex["stats"],
            }
            # print(hlprint(self.enc, odm["orig_tokens"][fn]))
            # print(odm["dest"][fn])
            diff = contrast.ContrastDiff(self.enc)
            diff.from_odm_dict(
                odm,
                n_ctx=1024,
            )
            assert len(diff.edits) == 1
            diff.write_edits()
            ex["prompt_tokens"] = diff.r[:diff.offset_first_postoken]
            ex["completion_tokens"] = diff.r[diff.offset_first_postoken:]
            odm["orig_tokens"] = diff.orig_tokens
            ex["decode_result_fn"] = functools.partial(_human_diff_decode_result, self.enc, odm)
            assert ex["decode_result_fn"](diff.r)["completion"] == dest
            yield ex


class HumanEvalInstructPrompt:
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
            fn = ex["entry_point"] + ".py"
            dest = ex["prompt"] + ex["canonical_solution"]
            odm = {
                "orig": {fn: ex["prompt"].rstrip() + "\n    pass\n\n\n"},
                "dest": {fn: dest},
                "commitmsg": "Implement %s() function" % ex["entry_point"],
                "original_prompt": ex["prompt"].rstrip(),
                "stats": ex["stats"],
            }
            # print(hlprint(self.enc, odm["orig_tokens"][fn]))
            # print(odm["dest"][fn])
            diff = contrast.ContrastDiff(self.enc)
            diff.from_odm_dict(
                odm,
                n_ctx=1024,
            )
            assert len(diff.edits) == 1
            diff.write_edits()
            ex["prompt_tokens"] = diff.r[:diff.offset_first_postoken]
            ex["completion_tokens"] = diff.r[diff.offset_first_postoken:]
            odm["orig_tokens"] = diff.orig_tokens
            ex["decode_result_fn"] = functools.partial(_human_diff_decode_result, self.enc, odm)
            assert ex["decode_result_fn"](diff.r)["completion"] == dest
            yield ex

