import functools

from code_contrast.format_2022q3 import contrast
from refact_encoding import RefactEncoding
from refact_data_pipeline import DatasetOpts

from typing import List


def postprocess(text: str, language: str):
    cutted = []
    if language in ["python"]:
        for x in text.split("\n"):
            if x.startswith(" ") or x.strip() == "":
                cutted.append(x)
            elif not x.startswith(" "):
                break
    elif language in ["cpp", "go", "js"]:
        for x in text.split("\n"):
            cutted.append(x)
            if x.startswith("}"):
                break
        else:
            cutted.append("}")
    elif language in ["java"]:
        for x in text.split("\n"):
            cutted.append(x)
            if x.startswith("    }"):
                break
        cutted.append("}")
    else:
        assert False, "language is not supported"
    return "\n".join(cutted)


class HumanEvalXContinuation:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
            language: str,
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc: RefactEncoding = dataopts.encoding
        self.language = language

    def decode_result(self, prompt, tokens_with_completion: List[int]):
        txt = self.enc.decode(tokens_with_completion, cut_at_eot=True)
        assert txt.startswith(prompt)
        completion = postprocess(txt[len(prompt):], self.language)
        ret = {
            "completion": completion,
            "tokens_with_completion": [int(t) for t in tokens_with_completion],
        }
        return ret

    def __iter__(self):
        for ex in self.inner_filter:
            ex["completion"] = ex["canonical_solution"]
            ex["decode_result_fn"] = functools.partial(self.decode_result, ex["prompt"])
            yield ex


class HumanEvalXContrast:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
            language: str,
    ):
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc: RefactEncoding = dataopts.encoding
        self.language = language
        fn_ext = {
            "python": "py",
            "cpp": "cpp",
            "java": "java",
            "js": "js",
            "go": "go",
        }[language]
        self.fn = f"test.{fn_ext}"

    def decode_result(self, odm, tokens_with_completion: List[int]):
        diff = contrast.ContrastDiff(self.enc)
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
        txt = self.enc.decode(dest_tokens.get(fn, []))
        if txt.startswith(odm["original_prompt"]):
            completion = postprocess(txt[len(odm["original_prompt"]):], self.language)
        else:
            completion = ""
        ret["errors"] = diff.errors
        ret["edit_stats"] = edit_stats
        ret["dest_tokens"] = diff.dest_tokens
        ret["completion"] = completion
        ret["tokens_with_completion"] = [int(t) for t in tokens_with_completion]
        # "completion_tokens" unchanged, will contain canonical solution
        return ret

    def __iter__(self):
        raise NotImplementedError


class HumanEvalXInfillPrompt(HumanEvalXContrast):

    def __iter__(self):
        for ex in self.inner_filter:
            dest = ex["prompt"] + ex["canonical_solution"]
            odm = {
                "orig": {self.fn: ex["prompt"] + self.enc.decode([self.enc.INFILL]) + "\n\n"},
                "dest": {self.fn: dest},
                "commitmsg": "Infill",
                "original_prompt": ex["prompt"].rstrip(),
                "stats": ex["stats"],
            }
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
            ex["decode_result_fn"] = functools.partial(self.decode_result, odm)
            yield ex


class HumanEvalXInstructPrompt(HumanEvalXContrast):

    def __iter__(self):
        for ex in self.inner_filter:
            dest = ex["prompt"] + ex["canonical_solution"]
            odm = {
                "orig": {self.fn: ex["prompt"].rstrip() + "\n\n\n"},
                "dest": {self.fn: dest},
                "commitmsg": f"Implement {ex['entry_point']} function",
                "original_prompt": ex["prompt"].rstrip(),
                "stats": ex["stats"],
            }
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
            ex["decode_result_fn"] = functools.partial(self.decode_result, odm)
            yield ex

