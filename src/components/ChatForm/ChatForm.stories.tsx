import type { Meta, StoryObj } from "@storybook/react";

import { ChatForm } from "./ChatForm";

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
const meta = {
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
      available_caps: ["bar", "baz"],
    },
    error: null,
    clearError: noop,
    canChangeModel: true,
    hasContextFile: false,
    handleContextFile: noop,
    commands: testCommands,
  },
} satisfies Meta<typeof ChatForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    model: "foo",
  },
};
