import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import {
  MARS_ROVER_CHAT,
  CHAT_FUNCTIONS_MESSAGES,
  FROG_CHAT,
  CHAT_WITH_DIFFS,
  CHAT_WITH_DIFF_ACTIONS,
  LARGE_DIFF,
} from "../../__fixtures__";

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
    addOrRemoveDiff: noop,
    getDiffByIndex: () => null,
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

export const WithDiffs: Story = {
  args: {
    messages: CHAT_WITH_DIFFS,
  },
};

export const WithDiffActions = {
  args: {
    messages: CHAT_WITH_DIFF_ACTIONS.messages,
    getDiffByIndex: (index: number) =>
      CHAT_WITH_DIFF_ACTIONS.applied_diffs["diff-" + index],
  },
};

export const LargeDiff = {
  args: {
    messages: LARGE_DIFF.messages,
    getDiffByIndex: (index: number) =>
      LARGE_DIFF.applied_diffs["diff-" + index],
  },
};
