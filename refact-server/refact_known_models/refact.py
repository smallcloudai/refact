refact_supports_scratchpads = {
    "FIM-SPM": {},
}

refact_mini_db = {
    "Refact/1.6B": {
        "backend": "transformers",
        "model_path": "smallcloudai/Refact-1_6B-fim",
        "model_class_kwargs": {
            "torch_dtype": "fp16",
        },
        "T": 4096,
        "required_memory_mb": 6000,
        "supports_scratchpads": {
            "completion": refact_supports_scratchpads,
        },
        "deprecated": True,
        "filter_caps": ["completion", "finetune"],
    },
    # "Refact/1.6B/cpu": {
    #     "backend": "transformers",
    #     "model_path": "smallcloudai/Refact-1_6B-fim",
    #     "model_class_kwargs": {
    #         "torch_dtype": "fp16",
    #     },
    #     "T": 4096,
    #     "cpu": True,
    #     "filter_caps": ["completion", "finetune"],
    # },
}
