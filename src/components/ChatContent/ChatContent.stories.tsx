import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import {
  MARS_ROVER_CHAT,
  CHAT_FUNCTIONS_MESSAGES,
  FROG_CHAT,
} from "../../__fixtures__";
import { ConfigProvider } from "../../contexts/config-context";

const noop = () => ({});

const meta = {
  title: "Chat Content",
  component: ChatContent,
  args: {
    messages: MARS_ROVER_CHAT.messages,
    onRetry: noop,
    isWaiting: false,
    isStreaming: false,
    canPaste: true,
    onNewFileClick: noop,
    onPasteClick: noop,
    chatKey: "test",
  },
} satisfies Meta<typeof ChatContent>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const WithFunctions: Story = {
  args: {
    ...meta.args,
    messages: CHAT_FUNCTIONS_MESSAGES,
  },
};

export const Notes: Story = {
  args: {
    messages: FROG_CHAT.messages,
  },
};

export const Empty: Story = {
  args: {
    ...meta.args,
    messages: [],
  },
  decorators: [
    (Story) => (
      <ConfigProvider config={{ host: "ide" }}>
        <Story />
      </ConfigProvider>
    ),
  ],
};
