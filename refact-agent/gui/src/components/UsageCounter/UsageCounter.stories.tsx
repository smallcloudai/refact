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
} from "./UsageCounter.fixtures";

const MockedStore: React.FC<{ usage: Usage }> = ({ usage }) => {
  const store = setUpStore({
    config: {
      themeProps: {
        appearance: "dark",
      },
      host: "web",
      lspPort: 8001,
    },
  });

  return (
    <Provider store={store}>
      <AbortControllerProvider>
        <Theme accentColor="gray">
          <UsageCounter usage={usage} />
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

export const GPTUsageCounter: StoryObj<typeof UsageCounter> = {
  args: {
    usage: USAGE_COUNTER_STUB_GPT,
  },
};
export const AnthropicUsageCounter: StoryObj<typeof UsageCounter> = {
  args: {
    usage: USAGE_COUNTER_STUB_ANTHROPIC,
  },
};
