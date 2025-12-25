import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import {
  type PayloadWithIdAndTitle,
  type ChatThread,
  type PayloadWithId,
  type ToolUse,
  type ImageFile,
  IntegrationMeta,
  LspChatMode,
  PayloadWithChatAndMessageId,
  PayloadWithChatAndBoolean,
  PayloadWithChatAndNumber,
} from "./types";
import type { ToolConfirmationPauseReason } from "../../../services/refact";
import { type ChatMessages } from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
import { type SystemPrompts } from "../../../services/refact/prompts";
import { ChatHistoryItem } from "../../History/historySlice";
import { ideToolCallResponse } from "../../../hooks/useEventBusForIDE";
import {
  trajectoriesApi,
  trajectoryDataToChatThread,
} from "../../../services/refact";

export const newChatAction = createAction<Partial<ChatThread> | undefined>(
  "chatThread/new",
);

export const newIntegrationChat = createAction<{
  integration: IntegrationMeta;
  messages: ChatMessages;
  request_attempt_id: string;
}>("chatThread/newIntegrationChat");

export const setLastUserMessageId = createAction<PayloadWithChatAndMessageId>(
  "chatThread/setLastUserMessageId",
);

export const setIsNewChatSuggested = createAction<PayloadWithChatAndBoolean>(
  "chatThread/setIsNewChatSuggested",
);

export const setIsNewChatSuggestionRejected =
  createAction<PayloadWithChatAndBoolean>(
    "chatThread/setIsNewChatSuggestionRejected",
  );

export const backUpMessages = createAction<
  PayloadWithId & {
    messages: ChatThread["messages"];
  }
>("chatThread/backUpMessages");

export const setChatModel = createAction<string>("chatThread/setChatModel");
export const getSelectedChatModel = (state: RootState) =>
  state.chat.threads[state.chat.current_thread_id]?.thread.model ?? "";

export const setSystemPrompt = createAction<SystemPrompts>(
  "chatThread/setSystemPrompt",
);

export const removeChatFromCache = createAction<PayloadWithId>(
  "chatThread/removeChatFromCache",
);

export const restoreChat = createAction<ChatHistoryItem>(
  "chatThread/restoreChat",
);

export const updateOpenThread = createAction<{
  id: string;
  thread: Partial<ChatThread>;
}>("chatThread/updateOpenThread");

export const switchToThread = createAction<PayloadWithId>(
  "chatThread/switchToThread",
);

export const closeThread = createAction<PayloadWithId & { force?: boolean }>(
  "chatThread/closeThread",
);

export const setThreadPauseReasons = createAction<{
  id: string;
  pauseReasons: ToolConfirmationPauseReason[];
}>("chatThread/setPauseReasons");

export const clearThreadPauseReasons = createAction<PayloadWithId>(
  "chatThread/clearPauseReasons",
);

export const setThreadConfirmationStatus = createAction<{
  id: string;
  wasInteracted: boolean;
  confirmationStatus: boolean;
}>("chatThread/setConfirmationStatus");

export const addThreadImage = createAction<{ id: string; image: ImageFile }>(
  "chatThread/addImage",
);

export const removeThreadImageByIndex = createAction<{
  id: string;
  index: number;
}>("chatThread/removeImageByIndex");

export const resetThreadImages = createAction<PayloadWithId>(
  "chatThread/resetImages",
);

export const clearChatError = createAction<PayloadWithId>(
  "chatThread/clearError",
);

export const enableSend = createAction<PayloadWithId>("chatThread/enableSend");
export const setPreventSend = createAction<PayloadWithId>(
  "chatThread/preventSend",
);
export const setAreFollowUpsEnabled = createAction<boolean>(
  "chat/setAreFollowUpsEnabled",
);

export const setUseCompression = createAction<boolean>(
  "chat/setUseCompression",
);

export const setToolUse = createAction<ToolUse>("chatThread/setToolUse");

export const setEnabledCheckpoints = createAction<boolean>(
  "chat/setEnabledCheckpoints",
);

export const setBoostReasoning = createAction<PayloadWithChatAndBoolean>(
  "chatThread/setBoostReasoning",
);

export const setAutomaticPatch = createAction<PayloadWithChatAndBoolean>(
  "chatThread/setAutomaticPatch",
);

export const saveTitle = createAction<PayloadWithIdAndTitle>(
  "chatThread/saveTitle",
);

export const setSendImmediately = createAction<boolean>(
  "chatThread/setSendImmediately",
);

export type EnqueueUserMessagePayload = {
  id: string;
  message: import("../../../services/refact/types").UserMessage;
  createdAt: number;
};

export const enqueueUserMessage = createAction<
  EnqueueUserMessagePayload & { priority?: boolean }
>("chatThread/enqueueUserMessage");

export const dequeueUserMessage = createAction<{ queuedId: string }>(
  "chatThread/dequeueUserMessage",
);

export const clearQueuedMessages = createAction(
  "chatThread/clearQueuedMessages",
);

export const setChatMode = createAction<LspChatMode>("chatThread/setChatMode");

export const setIntegrationData = createAction<Partial<IntegrationMeta> | null>(
  "chatThread/setIntegrationData",
);

export const setIsWaitingForResponse = createAction<{ id: string; value: boolean }>(
  "chatThread/setIsWaiting",
);

export const setMaxNewTokens = createAction<number>(
  "chatThread/setMaxNewTokens",
);

export const fixBrokenToolMessages = createAction<PayloadWithId>(
  "chatThread/fixBrokenToolMessages",
);

export const upsertToolCall = createAction<
  Parameters<typeof ideToolCallResponse>[0] & { replaceOnly?: boolean }
>("chatThread/upsertToolCall");

export const setIncreaseMaxTokens = createAction<boolean>(
  "chatThread/setIncreaseMaxTokens",
);

export const setIncludeProjectInfo = createAction<PayloadWithChatAndBoolean>(
  "chatThread/setIncludeProjectInfo",
);

export const setContextTokensCap = createAction<PayloadWithChatAndNumber>(
  "chatThread/setContextTokensCap",
);

export const restoreChatFromBackend = createAsyncThunk<
  void,
  { id: string; fallback: ChatHistoryItem },
  { dispatch: AppDispatch; state: RootState }
>(
  "chatThread/restoreChatFromBackend",
  async ({ id, fallback }, thunkApi) => {
    try {
      const result = await thunkApi.dispatch(
        trajectoriesApi.endpoints.getTrajectory.initiate(id, {
          forceRefetch: true,
        }),
      ).unwrap();

      const thread = trajectoryDataToChatThread(result);
      const historyItem: ChatHistoryItem = {
        ...thread,
        createdAt: result.created_at,
        updatedAt: result.updated_at,
        title: result.title,
        isTitleGenerated: result.isTitleGenerated,
      };

      thunkApi.dispatch(restoreChat(historyItem));
    } catch {
      // Backend not available, use fallback from history
      thunkApi.dispatch(restoreChat(fallback));
    }
  },
);

import type { ChatEventEnvelope } from "../../../services/refact/chatSubscription";

export const applyChatEvent = createAction<ChatEventEnvelope>(
  "chatThread/applyChatEvent",
);

export type IdeToolRequiredPayload = {
  chatId: string;
  toolCallId: string;
  toolName: string;
  args: unknown;
};

export const ideToolRequired = createAction<IdeToolRequiredPayload>(
  "chatThread/ideToolRequired",
);
