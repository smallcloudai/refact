import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { MessageNode } from "./MessageNode";
import { CMESSAGES_WITH_NESTED_BRANCHES_STUB } from "../../__fixtures__";
import { makeMessageTree } from "../../features/ChatDB/makeMessageTree";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import { setUpStore } from "../../app/store";
import { CMessageNode } from "../../features/ChatDB/chatDbMessagesSlice";

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const messageTree = makeMessageTree(CMESSAGES_WITH_NESTED_BRANCHES_STUB)!;

const Template: React.FC<{ node: CMessageNode }> = ({ node }) => {
  const store = setUpStore();

  return (
    <Provider store={store}>
      <Theme>
        <AbortControllerProvider>
          <MessageNode>{node}</MessageNode>
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
