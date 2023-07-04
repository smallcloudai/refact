big_code_mini_db = {
"starcoder/santacoder": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/santacoder",
    "diff_scratchpad_class": "code_contrast:ScratchpadBigCode",
    "chat_scratchpad_class": None,
    "model_class": "refact_models:HFModel",
    "T": 2048,
    "filter_caps": ["santacoder"],
    "hidden": True,   # only for debugging because it doesn't work well
},

"starcoder/15b": {
    "model_path_type": "huggingface",
    "model_path": "bigcode/starcoder",
    "diff_scratchpad_class": "code_contrast:ScratchpadBigCode",
    "chat_scratchpad_class": "code_contrast:ScratchpadBigChat",
    "model_class": "refact_models:HFModel",
    "T": 2048,
    "filter_caps": ["starcoder"],
},

"starcoder/15b/base4bit": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/starcoder_15b_4bit",
    "diff_scratchpad_class": "code_contrast:ScratchpadBigCode",
    "chat_scratchpad_class": "code_contrast:ScratchpadBigChat",
    "model_class": "refact_models:GPTQBigCodeModel",
    "model_class_kwargs": {
        "bits": 4,
    },
    "T": 2048,
    "filter_caps": ["starcoder"],
},

"starcoder/15b/base8bit": {
    "model_path_type": "huggingface",
    "model_path": "smallcloudai/starcoder_15b_8bit",
    "diff_scratchpad_class": "code_contrast:ScratchpadBigCode",
    "chat_scratchpad_class": "code_contrast:ScratchpadBigChat",
    "model_class": "refact_models:GPTQBigCodeModel",
    "model_class_kwargs": {
        "bits": 8,
    },
    "T": 2048,
    "filter_caps": ["starcoder"],
},

"starchat/15b/beta8bit": {
    "model_path_type": "huggingface",
    "model_path": "rahuldshetty/starchat-beta-8bit",
    "diff_scratchpad_class": None,
    "chat_scratchpad_class": "code_contrast:ScratchpadStarChat",
    "model_class": "refact_models:StarChatModel",
    "T": 2048,
    "filter_caps": ["starchat"],
},
}
