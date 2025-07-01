import { RootState } from "../../../app/store";
import { createSelector } from "@reduxjs/toolkit";
import { isDiffMessage } from "../../../services/refact/types";

export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectToolUse = (state: RootState) => state.chat.tool_use;
export const selectThreadToolUse = (state: RootState) =>
  state.chat.thread.tool_use;
export const selectAutomaticPatch = (state: RootState) =>
  state.chat.thread.automatic_patch;

export const selectCheckpointsEnabled = (state: RootState) =>
  state.chat.checkpoints_enabled;

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
