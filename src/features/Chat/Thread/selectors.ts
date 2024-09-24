import { RootState } from "../../../app/store";

export const selectThread = (state: RootState) => state.chat.thread;
export const selectThreadTitle = (state: RootState) => state.chat.thread.title;
export const selectChatId = (state: RootState) => state.chat.thread.id;
export const selectModel = (state: RootState) => state.chat.thread.model;
export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectToolUse = (state: RootState) => state.chat.tool_use;
export const selectIsWaiting = (state: RootState) =>
  state.chat.waiting_for_response;
export const selectIsStreaming = (state: RootState) => state.chat.streaming;
export const selectPreventSend = (state: RootState) => state.chat.prevent_send;
export const selectChatError = (state: RootState) => state.chat.error;
export const selectSendImmediately = (state: RootState) =>
  state.chat.send_immediately;
export const getSelectedSystemPrompt = (state: RootState) =>
  state.chat.system_prompt;

export const getSelectedToolUse = (state: RootState) =>
  state.chat.thread.tool_use;
