import { RootState } from "../../../app/store";
import { createSelector } from "@reduxjs/toolkit";
import { isToolMessage } from "../../../services/refact/types";

export const selectThread = (state: RootState) => state.chat.thread;
export const selectThreadTitle = (state: RootState) => state.chat.thread.title;
export const selectChatId = (state: RootState) => state.chat.thread.id;
export const selectModel = (state: RootState) => state.chat.thread.model;
export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectToolUse = (state: RootState) => state.chat.tool_use;
export const selectThreadToolUse = (state: RootState) =>
  state.chat.thread.tool_use;
export const selectAutomaticPatch = (state: RootState) =>
  state.chat.automatic_patch;

export const selectCheckpointsEnabled = (state: RootState) =>
  state.chat.checkpoints_enabled;

export const selectThreadNewChatSuggested = (state: RootState) =>
  state.chat.thread.new_chat_suggested;
export const selectThreadUsage = (state: RootState) => state.chat.thread.usage;
export const selectIsWaiting = (state: RootState) =>
  state.chat.waiting_for_response;
export const selectIsStreaming = (state: RootState) => state.chat.streaming;
export const selectPreventSend = (state: RootState) => state.chat.prevent_send;
export const selectChatError = (state: RootState) => state.chat.error;
export const selectSendImmediately = (state: RootState) =>
  state.chat.send_immediately;
export const getSelectedSystemPrompt = (state: RootState) =>
  state.chat.system_prompt;

export const toolMessagesSelector = createSelector(
  selectMessages,
  (messages) => {
    return messages.filter(isToolMessage);
  },
);

export const selectToolResultById = createSelector(
  [toolMessagesSelector, (_, id?: string) => id],
  (messages, id) => {
    return messages.find((message) => message.content.tool_call_id === id)
      ?.content;
  },
);

export const selectManyToolResultsByIds = (ids: string[]) =>
  createSelector(toolMessagesSelector, (messages) => {
    return messages
      .filter((message) => ids.includes(message.content.tool_call_id))
      .map((toolMessage) => toolMessage.content);
  });

export const getSelectedToolUse = (state: RootState) =>
  state.chat.thread.tool_use;

export const selectIntegration = createSelector(
  selectThread,
  (thread) => thread.integration,
);

export const selectThreadMode = createSelector(
  selectThread,
  (thread) => thread.mode,
);
