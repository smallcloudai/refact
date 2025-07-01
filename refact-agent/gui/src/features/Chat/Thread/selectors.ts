import { RootState } from "../../../app/store";

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
