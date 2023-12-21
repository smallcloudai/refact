import { CapsResponse } from "../services/refact";

export const STUB_CAPS_RESPONSE: CapsResponse = {
  caps_version: 0,
  cloud_name: "Refact",
  code_chat_default_model: "gpt-3.5-turbo",
  code_chat_models: {
    "gpt-3.5-turbo": {
      default_scratchpad: "",
      n_ctx: 4096,
      similar_models: [],
      supports_scratchpads: {
        PASSTHROUGH: {
          default_system_message:
            "You are a coding assistant that outputs short answers, gives links to documentation.",
        },
      },
    },
    "test-model": {
      default_scratchpad: "",
      n_ctx: 4096,
      similar_models: [],
      supports_scratchpads: {
        PASSTHROUGH: {
          default_system_message:
            "You are a coding assistant that outputs short answers, gives links to documentation.",
        },
      },
    },
  },
  code_completion_default_model: "smallcloudai/Refact-1_6B-fim",
  code_completion_models: {
    "smallcloudai/Refact-1_6B-fim": {
      default_scratchpad: "FIM-SPM",
      n_ctx: 4096,
      similar_models: ["Refact/1.6B", "Refact/1.6B/vllm"],
      supports_scratchpads: {
        "FIM-PSM": {},
        "FIM-SPM": {},
      },
    },
  },
  code_completion_n_ctx: 2048,
  endpoint_chat_passthrough:
    "https://inference.smallcloud.ai/v1/chat/completions",
  endpoint_style: "openai",
  endpoint_template: "https://inference.smallcloud.ai/v1/completions",
  running_models: ["smallcloudai/Refact-1_6B-fim", "gpt-3.5-turbo"],
  telemetry_basic_dest: "https://www.smallcloud.ai/v1/telemetry-basic",
  telemetry_corrected_snippets_dest: "",
  tokenizer_path_template:
    "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
  tokenizer_rewrite_path: {},
};
