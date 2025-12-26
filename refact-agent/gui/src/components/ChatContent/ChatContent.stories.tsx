import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import { MarkdownMessage } from "../../__fixtures__/markdown";
import type { ChatMessages } from "../../services/refact";
import type { ChatThread } from "../../features/Chat/Thread";
import {
  CHAT_FUNCTIONS_MESSAGES,
  CHAT_WITH_DIFF_ACTIONS,
  CHAT_WITH_DIFFS,
  FROG_CHAT,
  LARGE_DIFF,
  CHAT_WITH_MULTI_MODAL,
  CHAT_CONFIG_THREAD,
  STUB_LINKS_FOR_CHAT_RESPONSE,
  CHAT_WITH_TEXTDOC,
  MARKDOWN_ISSUE,
} from "../../__fixtures__";
import { http, HttpResponse } from "msw";
import { CHAT_LINKS_URL } from "../../services/refact/consts";
import {
  goodCaps,
  goodPing,
  goodPrompts,
  goodUser,
  noCommandPreview,
  noCompletions,
  noTools,
  ToolConfirmation,
} from "../../__fixtures__/msw";

const MockedStore: React.FC<{
  messages?: ChatMessages;
  thread?: ChatThread;
}> = ({ messages, thread }) => {
  const threadData = thread ?? {
    id: "test",
    model: "test",
    messages: messages ?? [],
    new_chat_suggested: {
      wasSuggested: false,
    },
  };
  const threadId = threadData.id ?? "test";
  const store = setUpStore({
    chat: {
      current_thread_id: threadId,
      open_thread_ids: [threadId],
      threads: {
        [threadId]: {
          thread: threadData,
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
          queue_size: 0,
        },
      },
      max_new_tokens: 4096,
      tool_use: "quick",
      system_prompt: {},
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
    thread:
      CHAT_CONFIG_THREAD.threads[CHAT_CONFIG_THREAD.current_thread_id]?.thread,
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
        goodCaps,
        goodPing,
        goodPrompts,
        goodUser,
        // noChatLinks,
        noTools,

        ToolConfirmation,
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
        goodCaps,
        goodPing,
        goodPrompts,
        goodUser,
        // noChatLinks,
        noTools,

        ToolConfirmation,
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
        { role: "user", content: "call a tool and wait" },
        {
          role: "assistant",
          content: "",
          tool_calls: [
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
        goodCaps,
        goodPing,
        goodPrompts,
        goodUser,
        // noChatLinks,
        noTools,

        ToolConfirmation,
        noCompletions,
        noCommandPreview,
      ],
    },
  },
};
