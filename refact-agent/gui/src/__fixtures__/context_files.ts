import type { ChatContextFile } from "../services/refact";

const some_text = `import { CapsResponse } from "../services/refact";

export const STUB_CAPS_RESPONSE: CapsResponse = {
  caps_version: 0,
  cloud_name: "Refact",
  chat_default_model: "gpt-3.5-turbo",
  chat_models: {
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
  completion_default_model: "smallcloudai/Refact-1_6B-fim",
  completion_models: {
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
  tokenizer_path_template:
    "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
  tokenizer_rewrite_path: {},
};
`;

export const CONTEXT_FILES: ChatContextFile[] = [
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/index.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.stories.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/FilesPreview.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/CharForm.test.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/RetryForm.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.module.css",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
  {
    file_name:
      "/Users/refacts/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/Form.tsx",
    file_content: some_text,
    line1: 1,
    line2: 100,
  },
];
