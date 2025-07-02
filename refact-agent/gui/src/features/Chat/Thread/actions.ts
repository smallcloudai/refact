import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import {
  type PayloadWithId,
  // type ToolUse,
  IntegrationMeta,
  LspChatMode,
} from "./types";
import {
  // isCDInstructionMessage,
  // isToolCallMessage,
  // isToolMessage,
  // ToolCall,
  // ToolMessage,
  type ChatMessages,
  // type ChatResponse,
} from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
// import { formatMessagesForLsp, consumeStream } from "./utils";
// import {
//   DEFAULT_MAX_NEW_TOKENS,
//   sendChat,
// } from "../../../services/refact/chat";
// import { ToolCommand, toolsApi } from "../../../services/refact/tools";
// import { scanFoDuplicatesWith, takeFromEndWhile } from "../../../utils";
// import {
//   DetailMessageWithErrorType,
//   isDetailMessage,
// } from "../../../services/refact";

export const newIntegrationChat = createAction<{
  integration: IntegrationMeta;
  messages: ChatMessages;
  request_attempt_id: string;
}>("chatThread/newIntegrationChat");

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

export const setEnabledCheckpoints = createAction<boolean>(
  "chat/setEnabledCheckpoints",
);

export const setIntegrationData = createAction<Partial<IntegrationMeta> | null>(
  "chatThread/setIntegrationData",
);

// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

// TODO: add props for config chat

export const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    checkpointsEnabled?: boolean;
    mode?: LspChatMode; // used once for actions
    // TODO: make a separate function for this... and it'll need to be saved.
  }
>("chatThread/sendChat", () => {
  return Promise.reject("Not implemented");
});
