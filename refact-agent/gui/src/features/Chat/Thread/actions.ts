import { createAction } from "@reduxjs/toolkit";
import {
  type PayloadWithId,
  // type ToolUse,
  // IntegrationMeta,
  // LspChatMode,
} from "./types";
// import {
//   isCDInstructionMessage,
//   isToolCallMessage,
//   isToolMessage,
//   ToolCall,
//   ToolMessage,
//   type ChatMessages,
//   type ChatResponse,
// } from "../../../services/refact/types";
// import type { AppDispatch, RootState } from "../../../app/store";
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

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

export const setEnabledCheckpoints = createAction<boolean>(
  "chat/setEnabledCheckpoints",
);
