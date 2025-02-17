import functools
from typing import List

from refact_data_pipeline import DatasetOpts
from refact_data_pipeline.datadef import PipelineNode


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


class HumanEvalXContinuation(PipelineNode):
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
            language: str,
    ):
        super().__init__(dataopts)
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc = dataopts.encoding
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
