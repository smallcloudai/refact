from code_contrast import ScratchpadDiff
from code_contrast import ScratchpadBigCode
from code_contrast import CodifyModel
from code_contrast import HFModel
from .known_functions import *


models_mini_db = {
"CONTRASTcode/medium/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_medium_multi",
    "diff_scratchpad_class": ScratchpadDiff,
    "model_class": CodifyModel,
    "T": 2048,
    "longthink_functions": {"hl_and_fix": hl_and_fix,
                            "select_and_refactor": select_and_refactor}
},
"CONTRASTcode/3b/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_3b_multi",
    "diff_scratchpad_class": ScratchpadDiff,
    "model_class": CodifyModel,
    "T": 2048,
    "longthink_functions": {"hl_and_fix": hl_and_fix,
                            "select_and_refactor": select_and_refactor}
},
"bigcode/santacoder": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/santacoder",
    "diff_scratchpad_class": ScratchpadBigCode,
    "model_class": HFModel,
    "T": 1024,
},
}
