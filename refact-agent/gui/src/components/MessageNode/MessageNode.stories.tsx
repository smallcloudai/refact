import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import {
  CHAT_WITH_TEXTDOC,
  CHAT_WITH_KNOWLEDGE_TOOL,
  CHAT_WITH_MULTI_MODAL,
} from "../../__fixtures__";
import {
  makeMessageTrie,
  FTMMessage,
  EmptyNode,
} from "../../features/ThreadMessages/makeMessageTrie";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { setUpStore } from "../../app/store";
import type { ChatMessage } from "../../services/refact/types";
import { FTMMessageNode as FTMessageNode } from "../../features/ThreadMessages/makeMessageTrie";
import { MessageNode } from "./MessageNode";
import { STUB_ALICE_MESSAGES } from "../../__fixtures__/message_lists";

function chatMessagesToCMessages(chatMessages: ChatMessage[]): FTMMessage[] {
  const messagesWithSystemMessage: ChatMessage[] =
    chatMessages[0].ftm_role === "system"
      ? chatMessages
      : [
          { ftm_role: "system", ftm_content: "system message" },
          ...chatMessages,
        ];

  return messagesWithSystemMessage.map<FTMMessage>(
    (message: ChatMessage, index) => {
      const cmessage: FTMMessage = {
        ftm_alt: 0,
        ftm_num: index,
        ftm_prev_alt: message.ftm_role === "system" ? -1 : 0,
        ftm_belongs_to_ft_id: "test",
        ftm_role: message.ftm_role,
        ftm_content: message.ftm_content,
        ftm_tool_calls:
          "tool_calls" in message ? message.tool_calls : undefined,
        ftm_call_id: "",
        ftm_usage: "usage" in message ? message.usage : null,
        ftm_created_ts: Date.now(),
      };

      return cmessage;
    },
  );
}

const messageTree = makeMessageTrie(STUB_ALICE_MESSAGES);

const Template: React.FC<{ node: FTMessageNode | EmptyNode }> = ({ node }) => {
  const store = setUpStore();

  return (
    <Provider store={store}>
      <Theme>
        {node.value ? (
          <MessageNode>{node}</MessageNode>
        ) : (
          <div>Could not make tree</div>
        )}
      </Theme>
    </Provider>
  );
};
const meta: Meta<typeof Template> = {
  title: "components/MessageNode",
  component: Template,
};

export default meta;

export const Primary: StoryObj<typeof Template> = {
  args: { node: messageTree },
};

export const Textdoc: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTrie(chatMessagesToCMessages(CHAT_WITH_TEXTDOC.messages)),
  },
};

export const Knowledge: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTrie(CHAT_WITH_KNOWLEDGE_TOOL),
  },
};

export const MultiModal: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTrie(
      chatMessagesToCMessages(CHAT_WITH_MULTI_MODAL.messages),
    ),
  },
};
