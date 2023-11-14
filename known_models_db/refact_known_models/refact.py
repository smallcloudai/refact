# refact_mini_db = {
#     "Refact/1.6B": {
#         "backend": "transformers",
#         "model_path": "smallcloudai/Refact-1_6B-fim",
#         "diff_scratchpad_class": "refact_scratchpads:ScratchpadSPM",
#         # "chat_scratchpad_class": "refact_scratchpads:ScratchpadHuggingfaceRefact",
#         "chat_scratchpad_class": None,  # chat is temporarily disabled
#         "model_class_kwargs": {
#             "torch_dtype": "fp16",
#         },
#         "T": 4096,
#         "required_memory_mb": 6000,
#         "filter_caps": ["Refact", "completion", "finetune"],
#     },
# }

from utils import ModelSpec
from utils import model_specs_from_list

from typing import List


refact_specs: List[ModelSpec] = [
    *model_specs_from_list(
        name="Refact/1.6B", context_sizes=[4096],
        filter_caps=["Refact", "completion"],
        diff_scratchpad_class="refact_scratchpads:ScratchpadSPM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "smallcloudai/Refact-1_6B-fim",
                "model_class_kwargs": {"torch_dtype": "fp16"},
                "finetune": True,
                "default": True,
            },
            {
                "backend": "transformers",
                "model_path": "smallcloudai/Refact-1_6B-fim",
                "quantization": "q8",
                "model_class_kwargs": {"load_in_8bit": True},
            },
            {
                "backend": "transformers",
                "model_path": "smallcloudai/Refact-1_6B-fim",
                "quantization": "q4",
                "model_class_kwargs": {"load_in_4bit": True},
            },
        ]),
]
