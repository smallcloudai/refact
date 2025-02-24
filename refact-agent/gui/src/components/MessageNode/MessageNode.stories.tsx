import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { MessageNode } from "./MessageNode";
import {
  CMESSAGES_WITH_NESTED_BRANCHES_STUB,
  CHAT_WITH_TEXTDOC,
  CHAT_WITH_KNOWLEDGE_TOOL,
  CHAT_WITH_MULTI_MODAL,
} from "../../__fixtures__";
import { makeMessageTree } from "../../features/ChatDB/makeMessageTree";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import { setUpStore } from "../../app/store";
import { CMessageNode } from "../../features/ChatDB/chatDbMessagesSlice";

import type { ChatMessage, CMessage } from "../../services/refact/types";

export function chatMessagesToCMessages(
  chatMessages: ChatMessage[],
): CMessage[] {
  const messagesWithSystemMessage: ChatMessage[] =
    chatMessages[0].role === "system"
      ? chatMessages
      : [{ role: "system", content: "system message" }, ...chatMessages];

  return messagesWithSystemMessage.map((message: ChatMessage) => {
    const cmessage: CMessage = {
      cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
      cmessage_alt: 0,
      cmessage_num: 0,
      cmessage_prev_alt: message.role === "system" ? -1 : 0,
      cmessage_usage_model: "",
      cmessage_usage_prompt: 0,
      cmessage_usage_completion: 0,
      cmessage_json: message,
    };

    return cmessage;
  });
}

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const messageTree = makeMessageTree(CMESSAGES_WITH_NESTED_BRANCHES_STUB)!;

const Template: React.FC<{ node: CMessageNode | null }> = ({ node }) => {
  const store = setUpStore();

  return (
    <Provider store={store}>
      <Theme>
        <AbortControllerProvider>
          {node ? (
            <MessageNode>{node}</MessageNode>
          ) : (
            <div>Could not make tree</div>
          )}
        </AbortControllerProvider>
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
    node: makeMessageTree(chatMessagesToCMessages(CHAT_WITH_TEXTDOC.messages)),
  },
};

export const Knowledge: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTree(
      chatMessagesToCMessages(CHAT_WITH_KNOWLEDGE_TOOL.messages),
    ),
  },
};

export const MultiModal: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTree(
      chatMessagesToCMessages(CHAT_WITH_MULTI_MODAL.messages),
    ),
  },
};
