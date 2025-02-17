import functools
from typing import List

from refact_data_pipeline import DatasetOpts
from refact_data_pipeline.datadef import PipelineNode


class HumanEvalContinuation(PipelineNode):
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts
    ):
        super().__init__(dataopts)
        self.inner_filter = inner_filter
        self.dataopts = dataopts
        self.enc = dataopts.encoding

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
