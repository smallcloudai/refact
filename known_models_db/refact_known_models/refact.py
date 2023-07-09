refact_mini_db = {
"CONTRASTcode/medium/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_medium_multi",
    "diff_scratchpad_class": "refact_scratchpads:ScratchpadDiff",
    "chat_scratchpad_class": None,
    "model_class": "refact_models:CodifyModel",
    "T": 2048,
    "filter_caps": ["CONTRASTcode", "completion"],
},

"CONTRASTcode/3b/multi": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/codify_3b_multi",
    "diff_scratchpad_class": "refact_scratchpads:ScratchpadDiff",
    "chat_scratchpad_class": None,
    "model_class": "refact_models:CodifyModel",
    "T": 2048,
    "filter_caps": ["CONTRASTcode", "completion", "finetune"],
},
}
