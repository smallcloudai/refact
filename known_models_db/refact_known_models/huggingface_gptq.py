huggingface_gptq_mini_db = {
    "wizardcoder/15b/1.0": {
        "model_path": "TheBloke/WizardCoder-15B-1.0-GPTQ",
        "diff_scratchpad_class": "refact_scratchpads:ScratchpadHuggingface",
        "chat_scratchpad_class": None,
        "model_class_kwargs": {},
        "filter_caps": ["completion"],
    },
    "starcoder/15b/base": {
        "model_path": "TheBloke/starcoder-GPTQ",
        "diff_scratchpad_class": "refact_scratchpads:ScratchpadHuggingface",
        "chat_scratchpad_class": None,
        "model_class_kwargs": {},
        "filter_caps": ["completion"],
    },
    "starcoder/15b/plus": {
        "model_path": "TheBloke/starcoderplus-GPTQ",
        "diff_scratchpad_class": "refact_scratchpads:ScratchpadHuggingface",
        "chat_scratchpad_class": None,
        "model_class_kwargs": {},
        "filter_caps": ["completion"],
    },
    "starchat/15b/beta": {
        "model_path": "TheBloke/starchat-beta-GPTQ",
        "diff_scratchpad_class": None,
        "chat_scratchpad_class": "refact_scratchpads:ScratchpadHuggingfaceStarChat",
        "model_class_kwargs": {},
        "filter_caps": ["starchat"],
    },
}
