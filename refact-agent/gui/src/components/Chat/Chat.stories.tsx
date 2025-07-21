import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Chat } from "./Chat";
// import { ChatThread } from "../../features/Chat/Thread/types";
import { RootState, setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import {
  CHAT_CONFIG_THREAD,
  // CHAT_WITH_KNOWLEDGE_TOOL,
} from "../../__fixtures__";

import {
  goodPing,
  goodUser,
  chatLinks,
  goodTools,
  noTools,
  // noChatLinks,
} from "../../__fixtures__/msw";
import { TourProvider } from "../../features/Tour";
import { Flex } from "@radix-ui/themes";
import { http, HttpResponse } from "msw";
import { BaseMessage } from "../../services/refact/types";

const Template: React.FC<{
  messages: BaseMessage[];
  config?: RootState["config"];
}> = ({ config, messages }) => {
  const store = setUpStore({
    tour: {
      type: "finished",
    },
    threadMessages: {
      waitingBranches: [],
      streamingBranches: [],
      ft_id: null,
      endNumber: 0,
      endAlt: 0,
      endPrevAlt: 0,
      thread: null,
      messages: messages.reduce((acc, message) => {
        return {
          ...acc,
          [message.ftm_call_id]: message,
        };
      }, {}),
    },
    config,
  });

  return (
    <Provider store={store}>
      <Theme>
        <TourProvider>
          <Flex direction="column" align="stretch" height="100dvh">
            <Chat
              host="web"
              tabbed={false}
              backFromChat={() => ({})}
              maybeSendToSidebar={() => ({})}
            />
          </Flex>
        </TourProvider>
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof Template> = {
  title: "Chat",
  component: Template,
  parameters: {
    msw: {
      handlers: [goodPing, goodUser, chatLinks, goodTools],
    },
  },
  argTypes: {},
};

export default meta;

type Story = StoryObj<typeof Template>;

export const Primary: Story = {};

export const Configuration: Story = {
  args: {
    messages: CHAT_CONFIG_THREAD,
  },
};

export const IDE: Story = {
  args: {
    config: {
      host: "ide",
      lspPort: 8001,
      themeProps: {},
      features: { vecdb: true },
    },
  },

  parameters: {
    msw: {
      handlers: [goodPing, goodUser, chatLinks, noTools],
    },
  },
};

export const Knowledge: Story = {
  args: {
    // thread: CHAT_WITH_KNOWLEDGE_TOOL,
    config: {
      host: "ide",
      lspPort: 8001,
      themeProps: {},
      features: {
        vecdb: true,
      },
    },
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        chatLinks,
        noTools,
      ],
    },
  },
};

export const EmptySpaceAtBottom: Story = {
  args: {
    messages: [
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      // { ftm_role: "assistant", ftm_content: "👋" },
    ].map((message, index) => {
      return {
        ftm_belongs_to_ft_id: "test",
        ftm_num: index,
        ftm_alt: 100,
        ftm_prev_alt: 100,
        ftm_created_ts: Date.now(),
        ftm_call_id: "",
        ...message,
      };
    }),
  },

  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        chatLinks,
        noTools,
      ],
    },
  },
};

export const UserMessageEmptySpaceAtBottom: Story = {
  args: {
    messages: [
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
    ].map((message, index) => {
      return {
        ftm_belongs_to_ft_id: "test",
        ftm_num: index,
        ftm_alt: 100,
        ftm_prev_alt: 100,
        ftm_created_ts: Date.now(),
        ftm_call_id: "",
        ...message,
      };
    }),
  },

  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        chatLinks,
        noTools,
      ],
    },
  },
};

export const CompressButton: Story = {
  args: {
    messages: [
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
      {
        ftm_role: "user",
        ftm_content: "Hello",
      },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
      },
      {
        ftm_role: "user",
        ftm_content: "👋",
        // change this to see different button colours
        compression_strength: "low",
      },
      { ftm_role: "assistant", ftm_content: "👋" },
    ].map((message, index) => {
      return {
        ftm_belongs_to_ft_id: "test",
        ftm_num: index,
        ftm_alt: 100,
        ftm_prev_alt: 100,
        ftm_created_ts: Date.now(),
        ftm_call_id: "",
        ...message,
      };
    }),
  },

  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        chatLinks,
        noTools,
      ],
    },
  },
};

const lowBalance = http.get("https://www.smallcloud.ai/v1/login", () => {
  return HttpResponse.json({
    retcode: "OK",
    account: "party@refact.ai",
    inference_url: "https://www.smallcloud.ai/v1",
    inference: "PRO",
    metering_balance: 1,
    questionnaire: {},
    refact_agent_max_request_num: 20,
    refact_agent_request_available: 20,
  });
});

export const LowBalance: Story = {
  parameters: {
    msw: {
      goodPing,

      chatLinks,
      noTools,
      lowBalance,
    },
  },
};
