import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import {
  CHAT_WITH_TEXTDOC,
  CHAT_WITH_KNOWLEDGE_TOOL,
  CHAT_WITH_MULTI_MODAL,
} from "../../__fixtures__";
import {
  makeMessageTrie,
  EmptyNode,
} from "../../features/ThreadMessages/makeMessageTrie";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { setUpStore } from "../../app/store";
import { FTMMessageNode as FTMessageNode } from "../../features/ThreadMessages/makeMessageTrie";
import { MessageNode } from "./MessageNode";
import { STUB_ALICE_MESSAGES } from "../../__fixtures__/message_lists";

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
    node: makeMessageTrie(CHAT_WITH_TEXTDOC),
  },
};

export const Knowledge: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTrie(CHAT_WITH_KNOWLEDGE_TOOL),
  },
};

export const MultiModal: StoryObj<typeof Template> = {
  args: {
    node: makeMessageTrie(CHAT_WITH_MULTI_MODAL),
  },
};
