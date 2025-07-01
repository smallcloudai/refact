import { RootState } from "../../../app/store";
import { createSelector } from "@reduxjs/toolkit";
import { isDiffMessage } from "../../../services/refact/types";

export const selectChatId = (state: RootState) => state.chat.thread.id;

export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectToolUse = (state: RootState) => state.chat.tool_use;
export const selectThreadToolUse = (state: RootState) =>
  state.chat.thread.tool_use;
export const selectAutomaticPatch = (state: RootState) =>
  state.chat.thread.automatic_patch;

export const selectCheckpointsEnabled = (state: RootState) =>
  state.chat.checkpoints_enabled;

export const selectThreadBoostReasoning = (state: RootState) =>
  state.chat.thread.boost_reasoning;

// TBD: only used when `/links` suggests a new chat.
export const selectThreadNewChatSuggested = (state: RootState) =>
  state.chat.thread.new_chat_suggested;
export const selectThreadMaximumTokens = (state: RootState) =>
  state.chat.thread.currentMaximumContextTokens;
export const selectThreadCurrentMessageTokens = (state: RootState) =>
  state.chat.thread.currentMessageContextTokens;
// export const selectIsWaiting = (state: RootState) =>
//   state.chat.waiting_for_response;

const selectDiffMessages = createSelector(selectMessages, (messages) =>
  messages.filter(isDiffMessage),
);

// TODO: needs migrated
export const selectManyDiffMessageByIds = (ids: string[]) =>
  createSelector(selectDiffMessages, (diffs) => {
    return diffs.filter((message) => ids.includes(message.tool_call_id));
  });
