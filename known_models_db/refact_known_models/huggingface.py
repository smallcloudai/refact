from known_models_db.refact_known_models.utils import ModelSpec
from known_models_db.refact_known_models.utils import model_specs_from_list

from typing import List


starcoder_specs: List[ModelSpec] = [
    *model_specs_from_list(
        name="starcoder/15b/base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/starcoder-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
    *model_specs_from_list(
        name="starcoder/15b/plus", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/starcoderplus-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
    *model_specs_from_list(
        name="starcoder/1b/base", context_sizes=[8192], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "smallcloudai/starcoderbase-1b",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
    *model_specs_from_list(
        name="starcoder/3b/base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "smallcloudai/starcoderbase-3b",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
    *model_specs_from_list(
        name="starcoder/7b/base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "smallcloudai/starcoderbase-7b",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
]

wizardcoder_specs = [
    *model_specs_from_list(
        name="wizardcoder/15b", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadPSM",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/WizardCoder-15B-1.0-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
]

codellama_specs = [
    *model_specs_from_list(
        name="codellama/7b", context_sizes=[2048], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadCodeLlamaSPM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "TheBloke/CodeLlama-7B-fp16",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
]

deepseek_specs = [
    *model_specs_from_list(
        name="deepseek-coder/1.3b/base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadDeepSeekCoderFIM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "deepseek-ai/deepseek-coder-1.3b-base",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
    *model_specs_from_list(
        name="deepseek-coder/5.7b/mqa-base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadDeepSeekCoderFIM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "deepseek-ai/deepseek-coder-5.7bmqa-base",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
    *model_specs_from_list(
        name="deepseek-coder/6.7b/base", context_sizes=[4096], completion=True,
        diff_scratchpad_class="refact_scratchpads:ScratchpadDeepSeekCoderFIM",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "deepseek-ai/deepseek-coder-6.7b-base",
                "finetune": True,
                "default": True, "default_finetune": True,
            },
        ]),
]

starchat_specs = [
    *model_specs_from_list(
        name="starchat/15b/beta", context_sizes=[4096], filter_caps=["starchat"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceStarChat",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/starchat-beta-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
]


wizardlm_specs = [
    *model_specs_from_list(
        name="wizardlm/7b", context_sizes=[2048], filter_caps=["wizardlm"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceWizard",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/WizardLM-7B-V1.0-Uncensored-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
    *model_specs_from_list(
        name="wizardlm/13b", context_sizes=[2048], filter_caps=["wizardlm"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceWizard",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/WizardLM-13B-V1.1-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
    *model_specs_from_list(
        name="wizardlm/30b", context_sizes=[2048], filter_caps=["wizardlm"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceWizard",
        specs_kwargs=[
            {
                "backend": "transformers",
                "model_path": "TheBloke/WizardLM-30B-fp16",
                "quantization": "4 bit",
                "model_class_kwargs": {"load_in_4bit": True},
                "default": True,
            },
        ]),
]

llama2_specs = [
    *model_specs_from_list(
        name="llama2/7b", context_sizes=[2048], filter_caps=["llama2"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceLlama2",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/Llama-2-7b-Chat-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
    *model_specs_from_list(
        name="llama2/13b", context_sizes=[2048], filter_caps=["llama2"],
        chat_scratchpad_class="refact_scratchpads:ScratchpadHuggingfaceLlama2",
        specs_kwargs=[
            {
                "backend": "autogptq",
                "model_path": "TheBloke/Llama-2-13B-chat-GPTQ",
                "quantization": "4 bit",
                "default": True,
            },
        ]),
]
