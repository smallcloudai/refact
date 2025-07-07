import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { MarkdownMessage } from "../../__fixtures__/markdown";
import type { ChatMessages } from "../../services/refact";
import type { ChatThread } from "../../features/Chat/Thread";
// TODO: update fixtures
import {
  // CHAT_FUNCTIONS_MESSAGES,
  // CHAT_WITH_DIFF_ACTIONS,
  // CHAT_WITH_DIFFS,
  // FROG_CHAT,
  // LARGE_DIFF,
  CHAT_WITH_MULTI_MODAL,
  CHAT_CONFIG_THREAD,
  STUB_LINKS_FOR_CHAT_RESPONSE,
  CHAT_WITH_TEXTDOC,
  MARKDOWN_ISSUE,
} from "../../__fixtures__";
import { http, HttpResponse } from "msw";
import { CHAT_LINKS_URL } from "../../services/refact/consts";
import {
  goodPing,
  goodUser,
  noCommandPreview,
  noCompletions,
  noTools,
} from "../../__fixtures__/msw";

const MockedStore: React.FC<{
  messages?: ChatMessages;
  thread?: ChatThread;
}> = () => {
  const store = setUpStore({});

  return (
    <Provider store={store}>
      <Theme>
        <ChatContent />
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
    // messages: CHAT_FUNCTIONS_MESSAGES,
    messages: [],
  },
};

export const Notes: Story = {
  args: {
    messages: [], // FROG_CHAT.messages,
  },
};

export const WithDiffs: Story = {
  args: {
    messages: [], // CHAT_WITH_DIFFS,
  },
};

export const WithDiffActions: Story = {
  args: {
    messages: [], // CHAT_WITH_DIFF_ACTIONS.messages,
    // getDiffByIndex: (key: string) => CHAT_WITH_DIFF_ACTIONS.applied_diffs[key],
  },
};

export const LargeDiff: Story = {
  args: {
    messages: [], // LARGE_DIFF.messages,
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
    messages: [{ ftm_role: "assistant", ftm_content: MarkdownMessage }],
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

export const TextDoc: Story = {
  args: {
    thread: CHAT_WITH_TEXTDOC,
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        noTools,

        noCompletions,
        noCommandPreview,
      ],
    },
  },
};

export const MarkdownIssue: Story = {
  args: {
    thread: MARKDOWN_ISSUE,
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        noTools,

        noCompletions,
        noCommandPreview,
      ],
    },
  },
};

export const ToolWaiting: Story = {
  args: {
    thread: {
      ...MARKDOWN_ISSUE,
      messages: [
        { ftm_role: "user", ftm_content: "call a tool and wait" },
        {
          ftm_role: "assistant",
          ftm_content: "",
          ftm_tool_calls: [
            {
              id: "toolu_01JbWarAwzjMyV6azDkd5skX",
              function: {
                arguments: '{"use_ast": true}',
                name: "tree",
              },
              type: "function",
              index: 0,
            },
          ],
        },
      ],
    },
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        noTools,

        noCompletions,
        noCommandPreview,
      ],
    },
  },
};
