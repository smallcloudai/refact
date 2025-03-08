import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import {
  CMessageFromChatDB,
  CThread,
  CMessage,
  ChatMessage,
  UserCMessage,
  isUserCMessage,
} from "../../services/refact";
import { parseOrElse } from "../../utils";
import { makeMessageTree } from "./makeMessageTree";
import { pagesSlice } from "../Pages/pagesSlice";

export interface CMessageNode {
  message: CMessage;
  children: CMessageNode[];
}

export type CMessageRoot = CMessageNode[];

export interface UserCMessageNode extends CMessageNode {
  message: UserCMessage;
}

export function isUserCMessageNode(
  node: CMessageNode,
): node is UserCMessageNode {
  return isUserCMessage(node.message);
}

type InitialState = {
  thread: Pick<CThread, "cthread_id" | "cthread_model" | "cthread_toolset">;
  messageList: CMessage[];
  loading: boolean;
  error: null | string;
  endNumber: number;
  endAlt: number;
};

const createChatThread = (): InitialState["thread"] => {
  const thread = {
    cthread_id: "",
    cthread_toolset: "",
    cthread_model: "",
  };
  return thread;
};

const initialState: InitialState = {
  thread: createChatThread(),
  messageList: [],
  loading: false,
  error: null,
  endNumber: 0,
  endAlt: 0,
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
    setThread: (state, action: PayloadAction<InitialState["thread"]>) => {
      state.thread = action.payload;
    },
    updateMessage: (
      state,
      action: PayloadAction<{ threadId: string; message: CMessageFromChatDB }>,
    ) => {
      if (action.payload.threadId !== state.thread.cthread_id) return state;
      const message = parseCMessageFromChatDBToCMessage(action.payload.message);
      if (!message) return;
      // Update message list
      const updateIndex = state.messageList.findIndex(
        (m) =>
          m.cmessage_num === message.cmessage_num &&
          m.cmessage_alt === message.cmessage_alt,
      );
      if (updateIndex > -1) {
        state.messageList[updateIndex] = message;
      } else {
        state.messageList.push(message);
        state.messageList.sort((a, b) => {
          if (a.cmessage_num === b.cmessage_num) {
            return a.cmessage_alt - b.cmessage_alt;
          }
          return a.cmessage_num - b.cmessage_num;
        });
      }
    },
    setEnd: (state, action: PayloadAction<{ number: number; alt: number }>) => {
      state.endNumber = action.payload.number;
      state.endAlt = action.payload.alt;
    },
  },

  extraReducers(builder) {
    // TODO: maybe move this
    builder.addMatcher(pagesSlice.actions.push.match, (state, action) => {
      if (action.payload.name !== "chat") return state;
      if (action.payload.threadId !== undefined) return state;
      const thread = createChatThread();
      thread.cthread_model = state.thread.cthread_model;
      thread.cthread_toolset = state.thread.cthread_toolset;
      return { ...initialState, thread };
    });
  },

  selectors: {
    selectMessageTree: (state) => makeMessageTree(state.messageList),
    selectThread: (state) => state.thread,
    selectThreadId: (state) => state.thread.cthread_id,
    selectLeafEndPosition: (state) => ({
      num: state.endNumber,
      alt: state.endAlt,
    }),
  },
});

export const chatDbMessageSliceActions = chatDbMessageSlice.actions;
export const chatDbMessagesSliceSelectors = chatDbMessageSlice.selectors;
