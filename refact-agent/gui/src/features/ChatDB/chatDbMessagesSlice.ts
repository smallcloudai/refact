import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import {
  CMessageFromChatDB,
  CThread,
  CThreadDefault,
  CMessage,
  ChatMessage,
} from "../../services/refact";
import { v4 as uuid } from "uuid";
import { parseOrElse } from "../../utils";

export type CMessageNode = {
  message: CMessage;
  children: CMessageNode[];
};

export type CMessageRoot = CMessageNode[];

type InitialState = {
  thread: CThread | CThreadDefault;
  messageTree: CMessageRoot;
  messageList: CMessage[];
  loading: boolean;
  error: null | string;
};

const createChatThread = (): CThreadDefault => {
  const thread: CThreadDefault = {
    cthread_id: uuid(),
    cthread_title: "",
    cthread_toolset: "",
    cthread_model: "",
  };

  return thread;
};

const initialState: InitialState = {
  thread: createChatThread(),
  messageTree: [[]],
  messageList: [],
  loading: false,
  error: null,
};

const findNodeByAltAndNum = (
  nodes: CMessageNode[],
  alt: number,
  num: number,
): CMessageNode | null => {
  for (const node of nodes) {
    if (
      node.message.cmessage_alt === alt &&
      node.message.cmessage_num === num
    ) {
      return node;
    }
    const found = findNodeByAltAndNum(node.children, alt, num);
    if (found) return found;
  }
  return null;
};

function parseCMessageFromChatDBToCMessage(
  message: CMessageFromChatDB,
): CMessage | null {
  // TODO: add a type guard to parseOrElse
  const json = parseOrElse<ChatMessage | null>(message.cmessage_json, null);
  if (json === null) return null;
  return {
    ...message,
    cmessage_json: json,
  };
}

export const chatDbMessageSlice = createSlice({
  name: "chatDbMessages",
  initialState,
  reducers: {
    setThread: (state, action: PayloadAction<CThread>) => {
      state.thread = action.payload;
      state.messageTree = [];
    },
    updateMessage: (
      state,
      action: PayloadAction<{ threadId: string; message: CMessageFromChatDB }>,
    ) => {
      if (action.payload.threadId !== state.thread.cthread_id) return state;
      const message = parseCMessageFromChatDBToCMessage(action.payload.message);
      if (!message) return;

      // Update message list
      state.messageList[message.cmessage_num] = message;

      if (message.cmessage_num === 0) {
        state.messageTree[message.cmessage_num] = {
          message,
          children: state.messageTree[message.cmessage_num]?.children ?? [],
        };
        return;
      }

      // find it's place
      //   function traverse(node: CMessageNode) {}
      // updateMessage: (
      //   state,
      //   action: PayloadAction<{ threadId: string; message: CMessageFromChatDB }>,
      // ) => {
      //   if (action.payload.threadId !== state.thread.cthread_id) return state;
      //   const message = parseCMessageFromChatDBToCMessage(action.payload.message);
      //   if (!message) return;

      //   state.messageList[message.cmessage_num] = message;
      //   if (message.cmessage_num === 0) {
      //     state.messageTree[message.cmessage_num] = {
      //       message,
      //       children: state.messageTree[message.cmessage_num]?.children ?? [],
      //     };

      //     return;
      //   }

      //   // find the parent node,
      //   const parentMessage = state.messageList.find(
      //     (m) =>
      //       m.cmessage_num === message.cmessage_num - 1 &&
      //       m.cmessage_alt === message.cmessage_prev_alt,
      //   );
      //   console.log("parentMessage", JSON.stringify(parentMessage)

      //   //   const parentNode = findNodeByAltAndNum(
      //   //     state.messageTree.flat(),
      //   //     prevAlt,
      //   //     prevNum
      //   //   );

      //   //   state.messageTree[message.cmessage_num] =
      //   //     state.messageTree[message.cmessage_num] ?? [];
      //   //   state.messageTree[message.cmessage_num][message.cmessage_alt] = {
      //   //     children:
      //   //       state.messageTree[message.cmessage_num][message.cmessage_alt]
      //   //         ?.children ?? [],
      //   //     message,
      //   //  };
      //   //   const node = row[message.cmessage_alt] ?? { message, children: [] };
      // },
    },
  },
});

export const chatDbMessageSliceActions = chatDbMessageSlice.actions;
export const chatDbMessagesSliceSelectors = chatDbMessageSlice.selectors;
