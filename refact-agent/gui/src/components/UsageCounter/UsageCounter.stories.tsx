import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Provider } from "react-redux";

import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { AbortControllerProvider } from "../../contexts/AbortControllers";

import { UsageCounter } from ".";
import { Usage } from "../../services/refact";
import {
  USAGE_COUNTER_STUB_ANTHROPIC,
  USAGE_COUNTER_STUB_GPT,
  USAGE_COUNTER_STUB_INLINE,
} from "./UsageCounter.fixtures";
import { Flex } from "@radix-ui/themes";

const MockedStore: React.FC<{
  usage: Usage;
  isInline?: boolean;
  isMessageEmpty?: boolean;
  threadMaximumContextTokens?: number;
  currentMessageContextTokens?: number;
}> = ({
  usage,
  threadMaximumContextTokens,
  currentMessageContextTokens,
  isInline = false,
  isMessageEmpty = false,
}) => {
  const threadId = "test";
  const store = setUpStore({
    config: {
      themeProps: {
        appearance: "dark",
      },
      host: "web",
      lspPort: 8001,
    },
    chat: {
      current_thread_id: threadId,
      open_thread_ids: [threadId],
      threads: {
        [threadId]: {
          thread: {
            id: threadId,
            messages: [
              {
                role: "user",
                content: "Hello, how are you?",
              },
              {
                role: "assistant",
                content: "Test content",
                usage,
              },
            ],
            model: "claude-3-5-sonnet",
            mode: "AGENT",
            new_chat_suggested: {
              wasSuggested: false,
            },
            currentMaximumContextTokens: threadMaximumContextTokens,
            currentMessageContextTokens,
          },
          streaming: false,
          waiting_for_response: false,
          prevent_send: false,
          error: null,
          queued_messages: [],
          send_immediately: false,
          attached_images: [],
          confirmation: {
            pause: false,
            pause_reasons: [],
            status: { wasInteracted: false, confirmationStatus: true },
          },
        },
      },
      tool_use: "agent",
      system_prompt: {},
    },
  });

  return (
    <Provider store={store}>
      <AbortControllerProvider>
        <Theme accentColor="gray">
          <Flex align="center" justify="center" width="50dvw" height="100dvh">
            <UsageCounter isInline={isInline} isMessageEmpty={isMessageEmpty} />
          </Flex>
        </Theme>
      </AbortControllerProvider>
    </Provider>
  );
};

const meta: Meta<typeof MockedStore> = {
  title: "UsageCounter",
  component: MockedStore,
  args: {
    usage: USAGE_COUNTER_STUB_GPT,
  },
};

export default meta;

export const GPTUsageCounter: StoryObj<typeof MockedStore> = {
  args: {
    usage: USAGE_COUNTER_STUB_GPT,
  },
};
export const AnthropicUsageCounter: StoryObj<typeof MockedStore> = {
  args: {
    usage: USAGE_COUNTER_STUB_ANTHROPIC,
  },
};

export const InlineUsageCounterInChatForm: StoryObj<typeof MockedStore> = {
  args: {
    usage: USAGE_COUNTER_STUB_INLINE,
    isInline: true,
    threadMaximumContextTokens: 2000,
    currentMessageContextTokens: 10,
  },
};
