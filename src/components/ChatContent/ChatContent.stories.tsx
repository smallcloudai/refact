import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";

const meta = {
  title: "Chat Content",
  component: ChatContent,
  args: {},
} satisfies Meta<typeof ChatContent>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const WithFunctions: Story = {
  args: {
    ...meta.args,
    // messages: CHAT_FUNCTIONS_MESSAGES,
  },
};

export const Notes: Story = {
  args: {
    // messages: FROG_CHAT.messages,
  },
};

export const WithDiffs: Story = {
  args: {
    // messages: CHAT_WITH_DIFFS,
  },
};

export const WithDiffActions = {
  args: {
    // messages: CHAT_WITH_DIFF_ACTIONS.messages,
    // getDiffByIndex: (key: string) => CHAT_WITH_DIFF_ACTIONS.applied_diffs[key],
  },
};

export const LargeDiff = {
  args: {
    // messages: LARGE_DIFF.messages,
    // getDiffByIndex: (key: string) => LARGE_DIFF.applied_diffs[key],
  },
};

export const Empty: Story = {
  args: {
    ...meta.args,
  },
  decorators: [
    // TODO: use redux store
    (Story) => (
      // <ConfigProvider config={{ host: "ide" }}>
      <Story />
      // </ConfigProvider>
    ),
  ],
};

export const AssistantMarkdown: Story = {
  args: {
    ...meta.args,
    // messages: [["assistant", MarkdownTest]],
  },
};
