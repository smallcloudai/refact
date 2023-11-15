from known_models_db.refact_known_models.utils import ModelSpec
from known_models_db.refact_known_models.utils import model_specs_from_list

from typing import List


refact_specs: List[ModelSpec] = [
    *model_specs_from_list(
        name="Refact/1.6B", context_sizes=[4096], completion=True, filter_caps=["Refact"],
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
