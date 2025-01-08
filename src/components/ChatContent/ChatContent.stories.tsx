import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import { MarkdownMessage } from "../../__fixtures__/markdown";
import { ChatMessages, ChatThread } from "../../events";
import {
  CHAT_FUNCTIONS_MESSAGES,
  CHAT_WITH_DIFF_ACTIONS,
  CHAT_WITH_DIFFS,
  FROG_CHAT,
  LARGE_DIFF,
  CHAT_WITH_MULTI_MODAL,
  CHAT_CONFIG_THREAD,
  STUB_LINKS_FOR_CHAT_RESPONSE,
} from "../../__fixtures__";
import { http, HttpResponse } from "msw";
import { CHAT_LINKS_URL } from "../../services/refact/consts";

const MockedStore: React.FC<{
  messages?: ChatMessages;
  thread?: ChatThread;
}> = ({ messages, thread }) => {
  const threadData = thread ?? {
    id: "test",
    model: "test",
    messages: messages ?? [],
  };
  const store = setUpStore({
    chat: {
      streaming: false,
      prevent_send: false,
      waiting_for_response: false,
      max_new_tokens: 4096,
      tool_use: "quick",
      send_immediately: false,
      error: null,
      cache: {},
      system_prompt: {},
      thread: threadData,
    },
  });

  return (
    <Provider store={store}>
      <Theme>
        <AbortControllerProvider>
          <ChatContent onRetry={() => ({})} onStopStreaming={() => ({})} />
        </AbortControllerProvider>
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "Chat Content",
  component: MockedStore,
  args: {
    messages: [],
  },
} satisfies Meta<typeof MockedStore>;

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

export const WithDiffActions: Story = {
  args: {
    messages: CHAT_WITH_DIFF_ACTIONS.messages,
    // getDiffByIndex: (key: string) => CHAT_WITH_DIFF_ACTIONS.applied_diffs[key],
  },
};

export const LargeDiff: Story = {
  args: {
    messages: LARGE_DIFF.messages,
    // getDiffByIndex: (key: string) => LARGE_DIFF.applied_diffs[key],
  },
};

export const Empty: Story = {
  args: {
    ...meta.args,
  },
};

export const AssistantMarkdown: Story = {
  args: {
    ...meta.args,
    messages: [{ role: "assistant", content: MarkdownMessage }],
  },
};

export const ToolImages: Story = {
  args: {
    ...meta.args,
  },
};

export const MultiModal: Story = {
  args: {
    messages: CHAT_WITH_MULTI_MODAL.messages,
  },
};

export const IntegrationChat: Story = {
  args: {
    thread: CHAT_CONFIG_THREAD.thread,
  },
  parameters: {
    msw: {
      handlers: [
        http.post(`http://127.0.0.1:8001${CHAT_LINKS_URL}`, () => {
          return HttpResponse.json(STUB_LINKS_FOR_CHAT_RESPONSE);
        }),
      ],
    },
  },
};
