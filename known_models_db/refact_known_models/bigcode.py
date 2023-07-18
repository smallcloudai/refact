big_code_mini_db = {

"starcoder/santacoder": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/santacoder",
    "diff_scratchpad_class": "refact_scratchpads:ScratchpadBigCode",
    "chat_scratchpad_class": None,
    "model_class": "refact_models:HFModel",
    "T": 2048,
    "filter_caps": ["santacoder"],
    "hidden": True,   # only for debugging because it doesn't work well
},

}
