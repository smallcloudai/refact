import dataclasses
from typing import Optional


@dataclasses.dataclass
class CompletionModelRecord:
    model_name: str
    code_completion_scratchpad: str = "single_file_fim:SingleFileFIM"
    supports_stop: bool = True


db_code_completion_models = [
    CompletionModelRecord("bigcode/starcoder"),
    CompletionModelRecord("bigcode/tiny_starcoder_py", supports_stop=False),
]


_quick_lookup_dict = {x.model_name: x for x in db_code_completion_models}


def model_lookup(model_name) -> Optional[CompletionModelRecord]:
    if model_name == "":
        model_name = "bigcode/starcoder"
    return _quick_lookup_dict.get(model_name, None)

