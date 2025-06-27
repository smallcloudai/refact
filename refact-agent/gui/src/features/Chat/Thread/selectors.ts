import { RootState } from "../../../app/store";
import { createSelector } from "@reduxjs/toolkit";
import { isDiffMessage, isToolMessage } from "../../../services/refact/types";

export const selectThread = (state: RootState) => state.chat.thread;
export const selectThreadTitle = (state: RootState) => state.chat.thread.title;
export const selectChatId = (state: RootState) => state.chat.thread.id;
export const selectModel = (state: RootState) => state.chat.thread.model;
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
export const selectAreFollowUpsEnabled = (state: RootState) =>
  state.chat.follow_ups_enabled;
export const selectIsTitleGenerationEnabled = (state: RootState) =>
  state.chat.title_generation_enabled;
// export const selectIsStreaming = (state: RootState) => state.chat.streaming;
export const selectPreventSend = (state: RootState) => state.chat.prevent_send;
export const selectChatError = (state: RootState) => state.chat.error;
export const selectSendImmediately = (state: RootState) =>
  state.chat.send_immediately;

export const toolMessagesSelector = createSelector(
  selectMessages,
  (messages) => {
    return messages.filter(isToolMessage);
  },
);

export const selectToolResultById = createSelector(
  [toolMessagesSelector, (_, id?: string) => id],
  (messages, id) => {
    return messages.find((message) => message.ftm_call_id === id)?.ftm_content;
  },
);

export const selectManyToolResultsByIds = (ids: string[]) =>
  createSelector(toolMessagesSelector, (messages) => {
    return messages
      .filter((message) => ids.includes(message.ftm_call_id))
      .map((toolMessage) => toolMessage.ftm_content);
  });

const selectDiffMessages = createSelector(selectMessages, (messages) =>
  messages.filter(isDiffMessage),
);

export const selectDiffMessageById = createSelector(
  [selectDiffMessages, (_, id?: string) => id],
  (messages, id) => {
    return messages.find((message) => message.tool_call_id === id);
  },
);

export const selectManyDiffMessageByIds = (ids: string[]) =>
  createSelector(selectDiffMessages, (diffs) => {
    return diffs.filter((message) => ids.includes(message.tool_call_id));
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

export const selectLastSentCompression = createSelector(
  selectMessages,
  (_messages) => {
    // TODO: this
    // const lastCompression = messages.reduce<null | CompressionStrength>(
    //   (acc, message) => {
    //     if (isUserMessage(message) && message.compression_strength) {
    //       return message.compression_strength;
    //     }
    //     if (
    //       isToolMessage(message) &&
    //       message.ftm_content.compression_strength
    //     ) {
    //       return message.ftm_content.compression_strength;
    //     }
    //     return acc;
    //   },
    //   null,
    // );

    // return lastCompression;
    return null;
  },
);
