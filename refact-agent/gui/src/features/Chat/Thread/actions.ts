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
import {
  isAssistantMessage,
  isCDInstructionMessage,
  isToolCallMessage,
  isToolMessage,
  isUserMessage,
  ToolCall,
  ToolMessage,
  type ChatMessages,
  type ChatResponse,
} from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
import { type SystemPrompts } from "../../../services/refact/prompts";
import { formatMessagesForLsp, consumeStream } from "./utils";
import { sendChat } from "../../../services/refact/chat";
// import { ToolCommand, toolsApi } from "../../../services/refact/tools";
import { scanFoDuplicatesWith, takeFromEndWhile } from "../../../utils";
import { ChatHistoryItem } from "../../History/historySlice";
import { ideToolCallResponse } from "../../../hooks/useEventBusForIDE";
import {
  DetailMessageWithErrorType,
  isDetailMessage,
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

export const chatResponse = createAction<PayloadWithId & ChatResponse>(
  "chatThread/response",
);



export const chatAskedQuestion = createAction<PayloadWithId>(
  "chatThread/askQuestion",
);

export const setLastUserMessageId = createAction<PayloadWithChatAndMessageId>(
  "chatThread/setLastUserMessageId",
);

// TBD: only used when `/links` suggests a new chat.
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

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

// TODO: include history actions with this one, this could be done by making it a thunk, or use reduce-reducers.
export const doneStreaming = createAction<PayloadWithId>(
  "chatThread/doneStreaming",
);

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

// Update an already-open thread with fresh data from backend (used by subscription)
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

// TBD: maybe remove it's only used by a smart link.
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

// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

function checkForToolLoop(message: ChatMessages): boolean {
  const assistantOrToolMessages = takeFromEndWhile(message, (message) => {
    return (
      isToolMessage(message) ||
      isToolCallMessage(message) ||
      isCDInstructionMessage(message)
    );
  });

  if (assistantOrToolMessages.length === 0) return false;

  const toolCalls = assistantOrToolMessages.reduce<ToolCall[]>((acc, cur) => {
    if (!isToolCallMessage(cur)) return acc;
    return acc.concat(cur.tool_calls);
  }, []);

  if (toolCalls.length === 0) return false;

  const toolResults = assistantOrToolMessages.filter(isToolMessage);

  const hasDuplicates = scanFoDuplicatesWith(toolCalls, (a, b) => {
    const aResult: ToolMessage | undefined = toolResults.find(
      (message) => message.content.tool_call_id === a.id,
    );

    const bResult: ToolMessage | undefined = toolResults.find(
      (message) => message.content.tool_call_id === b.id,
    );

    return (
      a.function.name === b.function.name &&
      a.function.arguments === b.function.arguments &&
      !!aResult &&
      !!bResult &&
      aResult.content.content === bResult.content.content
    );
  });

  return hasDuplicates;
}
// TODO: add props for config chat

export const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    checkpointsEnabled?: boolean;
    mode?: LspChatMode;
  }
>(
  "chatThread/sendChat",
  ({ messages, chatId, mode, checkpointsEnabled }, thunkAPI) => {
    const state = thunkAPI.getState();

    const runtime = state.chat.threads[chatId];
    const thread = runtime?.thread ?? null;

    const onlyDeterministicMessages = checkForToolLoop(messages);

    const messagesForLsp = formatMessagesForLsp(messages);
    const realMode = mode ?? thread?.mode;
    const maybeLastUserMessageId = thread?.last_user_message_id;
    const boostReasoning = thread?.boost_reasoning ?? false;
    const increaseMaxTokens = thread?.increase_max_tokens ?? false;
    const userMessageCount = messages.filter(isUserMessage).length;
    const includeProjectInfo =
      userMessageCount <= 1 ? thread?.include_project_info ?? true : undefined;

    const contextTokensCap =
      thread?.context_tokens_cap ?? thread?.currentMaximumContextTokens;

    const useCompression = state.chat.use_compression;

    const model = thread?.model ?? "";

    return sendChat({
      messages: messagesForLsp,
      last_user_message_id: maybeLastUserMessageId,
      model,
      stream: true,
      abortSignal: thunkAPI.signal,
      increase_max_tokens: increaseMaxTokens,
      chatId,
      apiKey: state.config.apiKey,
      port: state.config.lspPort,
      onlyDeterministicMessages,
      checkpointsEnabled,
      integration: thread?.integration,
      mode: realMode,
      boost_reasoning: boostReasoning,
      include_project_info: includeProjectInfo,
      context_tokens_cap: contextTokensCap,
      use_compression: useCompression,
    })
      .then(async (response) => {
        if (!response.ok) {
          const responseData = (await response.json()) as unknown;
          return Promise.reject(responseData);
        }
        const reader = response.body?.getReader();
        if (!reader) return;
        const onAbort = () => {
          thunkAPI.dispatch(setPreventSend({ id: chatId }));
          thunkAPI.dispatch(fixBrokenToolMessages({ id: chatId }));
        };
        const onChunk = (json: Record<string, unknown>) => {
          const action = chatResponse({
            ...(json as ChatResponse),
            id: chatId,
          });
          return thunkAPI.dispatch(action);
        };
        return consumeStream(reader, thunkAPI.signal, onAbort, onChunk);
      })
      .catch((err: unknown) => {
        const isError = err instanceof Error;
        // Note: doneStreaming is called in .finally() - don't duplicate here
        thunkAPI.dispatch(fixBrokenToolMessages({ id: chatId }));

        const errorObject: DetailMessageWithErrorType = {
          detail: isError
            ? err.message
            : isDetailMessage(err)
              ? err.detail
              : (err as string),
          errorType: isError ? "CHAT" : "GLOBAL",
        };

        return thunkAPI.rejectWithValue(errorObject);
      })
      .finally(() => {
        thunkAPI.dispatch(doneStreaming({ id: chatId }));
      });
  },
);

export const sendCurrentChatToLspAfterToolCallUpdate = createAppAsyncThunk<
  unknown,
  { chatId: string; toolCallId: string }
>(
  "chatThread/sendCurrentChatToLspAfterToolCallUpdate",
  async ({ chatId, toolCallId }, thunkApi) => {
    const state = thunkApi.getState();
    const runtime = state.chat.threads[chatId];
    if (!runtime) return;

    if (runtime.streaming || runtime.prevent_send || runtime.waiting_for_response) {
      return;
    }

    const lastMessages = takeFromEndWhile(
      runtime.thread.messages,
      (message) => !isUserMessage(message) && !isAssistantMessage(message),
    );

    const toolUseInThisSet = lastMessages.some(
      (message) =>
        isToolMessage(message) && message.content.tool_call_id === toolCallId,
    );

    if (!toolUseInThisSet) return;
    thunkApi.dispatch(setIsWaitingForResponse({ id: chatId, value: true }));

    return thunkApi.dispatch(
      chatAskQuestionThunk({
        messages: runtime.thread.messages,
        chatId,
        mode: runtime.thread.mode,
        checkpointsEnabled: state.chat.checkpoints_enabled,
      }),
    );
  },
);

// Fetch fresh thread data from backend before restoring (re-opening a closed tab)
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
