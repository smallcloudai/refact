import type { Meta, StoryObj } from "@storybook/react";

import { ChatForm } from "./ChatForm";
import { SYSTEM_PROMPTS } from "../../__fixtures__";
import { ConfigProvider } from "../../contexts/config-context";
import { useDebounceCallback } from "usehooks-ts";

const testCommands = [
  "@workspace",
  "@help",
  "@list",
  "@web",
  "@database",
  "@?",
  "@longlonglonglonglonglonglonglonglonglong",
  "@refactor",
  "@test",
  "@Apple",
  "@Banana",
  "@Carrot",
  "@Dill",
  "@Elderberries",
  "@Figs",
  "@Grapes",
  "@Honeydew",
  "@Iced melon",
  "@Jackfruit",
  "@Kale",
  "@Lettuce",
  "@Mango",
  "@Nectarines",
  "@Oranges",
  "@Pineapple",
  "@Quince",
  "@Raspberries",
  "@Strawberries",
  "@Turnips",
  "@Ugli fruit",
  "@Vanilla beans",
  "@Watermelon",
  "@Xigua",
  "@Yuzu",
  "@Zucchini",
];

const noop = () => ({});

const long = "long".repeat(30);

const meta: Meta<typeof ChatForm> = {
  title: "Chat Form",
  component: ChatForm,
  args: {
    onSubmit: (str) => {
      // eslint-disable-next-line no-console
      console.log("submit called with " + str);
    },
    onClose: () => {
      // eslint-disable-next-line no-console
      console.log("onclose called");
    },
    isStreaming: false,
    onStopStreaming: noop,
    onSetChatModel: noop,
    caps: {
      fetching: false,
      default_cap: "foo",
      available_caps: {
        bar: {
          default_scratchpad: "",
          n_ctx: 2048,
          similar_models: [],
          supports_tools: false,
          supports_scratchpads: {},
        },
        [long]: {
          default_scratchpad: "",
          n_ctx: 2048,
          similar_models: [],
          supports_tools: false,
          supports_scratchpads: {},
        },
        baz: {
          default_scratchpad: "",
          n_ctx: 2048,
          similar_models: [],
          supports_tools: false,
          supports_scratchpads: {},
        },
      },
      error: "",
    },
    error: null,
    clearError: noop,
    showControls: true,
    hasContextFile: false,
    commands: {
      completions: testCommands,
      replace: [-1, -1],
      is_cmd_executable: false,
    },
    attachFile: {
      name: "todo.md",
      can_paste: true,
      attach: false,
      line1: 1,
      line2: 100,
      path: "/Users/refact/Projects/smallcloudai/refact-chat-js/todo.md",
      cursor: 50,
    },
    filesInPreview: [
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/index.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.stories.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/FilesPreview.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/CharForm.test.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/RetryForm.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.module.css",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refact/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/ChatForm.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
      {
        file_name:
          "/Users/refacts/Projects/smallcloudai/refact-chat-js/src/components/ChatForm/Form.tsx",
        file_content: "",
        line1: 1,
        line2: 100,
      },
    ],
    selectedSnippet: { code: "", language: "", basename: "", path: "" },
    removePreviewFileByName: () => ({}),
    requestPreviewFiles: () => ({}),
    // requestCommandsCompletion: () => ({}),
    // setSelectedCommand: () => ({}),
    onTextAreaHeightChange: noop,
    prompts: SYSTEM_PROMPTS,
    onSetSystemPrompt: noop,
    selectedSystemPrompt: null,
    useTools: true,
    canUseTools: true,
    setUseTools: noop,
  },
  decorators: [
    (Children) => {
      const requestCommandsCompletion = useDebounceCallback(() => ({}), 0);
      return (
        <ConfigProvider
          config={{ host: "vscode", features: { vecdb: true, ast: true } }}
        >
          <Children requestCommandsCompletion={requestCommandsCompletion} />
        </ConfigProvider>
      );
    },
  ],
} satisfies Meta<typeof ChatForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    model: "foo",
  },
};
