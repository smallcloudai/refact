from code_contrast import ScratchpadDiff
from code_contrast import ScratchpadBigCode
from code_contrast import ScratchpadBigChat

from code_contrast import CodifyModel
from code_contrast import HFModel
from code_contrast import GPTQBigCodeModel


models_mini_db = {
"CONTRASTcode/medium/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_medium_multi",
    "diff_scratchpad_class": ScratchpadDiff,
    "chat_scratchpad_class": None,
    "model_class": CodifyModel,
    "T": 2048,
    "filter_caps": ["CONTRASTcode"],
},
"CONTRASTcode/3b/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_3b_multi",
    "diff_scratchpad_class": ScratchpadDiff,
    "chat_scratchpad_class": None,
    "model_class": CodifyModel,
    "T": 2048,
    "filter_caps": ["CONTRASTcode"],
},
"bigcode/santacoder": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/santacoder",
    "diff_scratchpad_class": ScratchpadBigCode,
    "chat_scratchpad_class": None,
    "model_class": HFModel,
    "T": 1024,
    "filter_caps": ["starcoder"],
},
"starcoder/15b": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/starcoder",
    "diff_scratchpad_class": ScratchpadBigCode,
    "chat_scratchpad_class": ScratchpadBigChat,
    "model_class": HFModel,
    "T": 2048,
    "filter_caps": ["starcoder"],
},
# starcoder/15b/base4bit
# starcoder/15b/base8bit
"bigcode/large-model": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/large-model",
    "diff_scratchpad_class": ScratchpadBigCode,
    "model_class": GPTQBigCodeModel,
    "chat_scratchpad_class": None,
    "model_class_kwargs": {
        "checkpoint": "/home/user/starcoder-GPTQ/4-bit/model.pt",
        "bits": 4,
        "groupsize": 128,
    },
    "T": 1024,
    "filter_caps": ["starcoder"],
},
}
