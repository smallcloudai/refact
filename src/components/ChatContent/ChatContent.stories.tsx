import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import { MARS_ROVER_CHAT } from "../../__fixtures__";

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
  },
} satisfies Meta<typeof ChatContent>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
