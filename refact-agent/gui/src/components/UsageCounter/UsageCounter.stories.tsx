import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Provider } from "react-redux";

import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";

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
}> = ({ usage, isInline = false, isMessageEmpty = false }) => {
  const store = setUpStore({
    config: {
      themeProps: {
        appearance: "dark",
      },
      host: "web",
      lspPort: 8001,
    },
    threadMessages: {
      loading: false,
      thread: {
        ft_id: "foo",
        ft_need_user: -1,
        ft_need_assistant: -1,
        ft_fexp_id: "id:ask:1.0",
        located_fgroup_id: "0000000",
        ft_title: "test",
      },
      ft_id: "foo",
      streamingBranches: [],
      waitingBranches: [],
      endNumber: 2,
      endAlt: 100,
      endPrevAlt: 100,
      messages: {
        aa: {
          ftm_num: 1,
          ftm_alt: 100,
          ftm_prev_alt: 100,
          ftm_role: "user",
          ftm_content: "Hello, how are you?",
          ftm_belongs_to_ft_id: "foo",
          ftm_call_id: "1",
          ftm_created_ts: 0,
        },
        ab: {
          ftm_num: 2,
          ftm_alt: 100,
          ftm_prev_alt: 100,
          ftm_role: "assistant",
          ftm_content: "Test content",
          ftm_belongs_to_ft_id: "foo",
          ftm_call_id: "1",
          ftm_created_ts: 1,
          ftm_usage: usage,
        },
      },
    },
  });

  return (
    <Provider store={store}>
      <Theme accentColor="gray">
        <Flex align="center" justify="center" width="50dvw" height="100dvh">
          <UsageCounter isInline={isInline} isMessageEmpty={isMessageEmpty} />
        </Flex>
      </Theme>
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
