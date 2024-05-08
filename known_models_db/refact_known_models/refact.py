refact_mini_db = {
    "Refact/1.6B": {
        "backend": "transformers",
        "model_path": "smallcloudai/Refact-1_6B-fim",
        "model_class_kwargs": {
            "torch_dtype": "fp16",
        },
        "T": 4096,
        "required_memory_mb": 6000,
        "filter_caps": ["Refact", "completion", "finetune"],
    },
}
