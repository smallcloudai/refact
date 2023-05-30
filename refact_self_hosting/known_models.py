import re
from typing import Tuple
from code_contrast import ScratchpadDiff
from code_contrast import ScratchpadBigCode
from code_contrast import ScratchpadBigChat

from code_contrast.modeling import CodifyModel
from code_contrast.modeling import HFModel
from code_contrast.modeling import GPTQBigCodeModel


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
"starcoder/santacoder": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/santacoder",
    "diff_scratchpad_class": ScratchpadBigCode,
    "chat_scratchpad_class": None,
    "model_class": HFModel,
    "T": 2048,
    "filter_caps": ["santacoder"],
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
"starcoder/15b/base4bit": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/starcoder_15b_4bit",
    "diff_scratchpad_class": ScratchpadBigCode,
    "chat_scratchpad_class": ScratchpadBigChat,
    "model_class": GPTQBigCodeModel,
    "model_class_kwargs": {
        "bits": 4,
    },
    "T": 2048,
    "filter_caps": ["starcoder"],
},
"starcoder/15b/base8bit": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/starcoder_15b_8bit",
    "diff_scratchpad_class": ScratchpadBigCode,
    "chat_scratchpad_class": ScratchpadBigChat,
    "model_class": GPTQBigCodeModel,
    "model_class_kwargs": {
        "bits": 8,
    },
    "T": 2048,
    "filter_caps": ["starcoder"],
},
}


def resolve_model(model_name: str, cursor_file: str, function: str) -> Tuple[str, str]:
    """
    Allow client to specify less, including an empty string.
    """
    m_everything = model_name.split("/")
    m_company, m_size, m_specialization, m_version = tuple(m_everything + ["", "", "", ""])[:4]

    if m_company == "CONTRASTcode":
        if function == "":  # true for plain completion (not diff)
            pass
        else:
            regex = r"^(highlight|infill|diff-anywhere|diff-atcursor|diff-selection|edit-chain)$"
            m_match = re.fullmatch(regex, function)
            if not m_match:
                return "", "function must match %s" % regex
        if not m_specialization and cursor_file:
            # file extension -> specialization here
            pass
        if not m_size:
            m_size = "3b"
        if not m_specialization:
            m_specialization = "multi"

    if m_company == "starcoder":
        if not m_size:
            m_size = "15b"

    result = "/".join([m_company, m_size, m_specialization, m_version])
    result = result.rstrip("/")
    return result, ""
