import type { Meta, StoryObj } from "@storybook/react";

import { ChatForm } from "./ChatForm";

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
  },
} satisfies Meta<typeof ChatForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    model: "foo",
  },
};
