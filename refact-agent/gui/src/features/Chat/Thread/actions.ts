import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import {
  type ChatThread,
  type PayloadWithId,
  type ToolUse,
  IntegrationMeta,
  LspChatMode,
} from "./types";
import {
  isCDInstructionMessage,
  isToolCallMessage,
  isToolMessage,
  ToolCall,
  ToolMessage,
  type ChatMessages,
  type ChatResponse,
} from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
import { formatMessagesForLsp, consumeStream } from "./utils";
import {
  DEFAULT_MAX_NEW_TOKENS,
  sendChat,
} from "../../../services/refact/chat";
// import { ToolCommand, toolsApi } from "../../../services/refact/tools";
import { scanFoDuplicatesWith, takeFromEndWhile } from "../../../utils";
import {
  DetailMessageWithErrorType,
  isDetailMessage,
} from "../../../services/refact";

// TODO: move this as it's used in vscode
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

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

// TODO: include history actions with this one, this could be done by making it a thunk, or use reduce-reducers.
export const doneStreaming = createAction<PayloadWithId>(
  "chatThread/doneStreaming",
);

export const setPreventSend = createAction<PayloadWithId>(
  "chatThread/preventSend",
);
export const setAreFollowUpsEnabled = createAction<boolean>(
  "chat/setAreFollowUpsEnabled",
);
export const setIsTitleGenerationEnabled = createAction<boolean>(
  "chat/setIsTitleGenerationEnabled",
);

export const setToolUse = createAction<ToolUse>("chatThread/setToolUse");

export const setEnabledCheckpoints = createAction<boolean>(
  "chat/setEnabledCheckpoints",
);

export const setIntegrationData = createAction<Partial<IntegrationMeta> | null>(
  "chatThread/setIntegrationData",
);

// TBD: maybe remove it's only used by a smart link.
export const setMaxNewTokens = createAction<number>(
  "chatThread/setMaxNewTokens",
);

export const fixBrokenToolMessages = createAction<PayloadWithId>(
  "chatThread/fixBrokenToolMessages",
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
      (message) => message.ftm_call_id === a.id,
    );

    const bResult: ToolMessage | undefined = toolResults.find(
      (message) => message.ftm_call_id === b.id,
    );

    return (
      a.function.name === b.function.name &&
      a.function.arguments === b.function.arguments &&
      !!aResult &&
      !!bResult &&
      aResult.ftm_content === bResult.ftm_content
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
    mode?: LspChatMode; // used once for actions
    // TODO: make a separate function for this... and it'll need to be saved.
  }
>(
  "chatThread/sendChat",
  ({ messages, chatId, mode, checkpointsEnabled }, thunkAPI) => {
    const state = thunkAPI.getState();

    const thread =
      chatId in state.chat.cache
        ? state.chat.cache[chatId]
        : state.chat.thread.id === chatId
          ? state.chat.thread
          : null;

    // stops the stream
    const onlyDeterministicMessages = checkForToolLoop(messages);

    const messagesForLsp = formatMessagesForLsp(messages);
    const realMode = mode ?? thread?.mode;
    const maybeLastUserMessageId = thread?.last_user_message_id;
    const boostReasoning = thread?.boost_reasoning ?? false;
    const increaseMaxTokens = thread?.increase_max_tokens ?? false;

    return sendChat({
      messages: messagesForLsp,
      last_user_message_id: maybeLastUserMessageId,
      model: state.chat.thread.model,
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
        // console.log("Catch called");
        const isError = err instanceof Error;
        thunkAPI.dispatch(doneStreaming({ id: chatId }));
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
        thunkAPI.dispatch(setMaxNewTokens(DEFAULT_MAX_NEW_TOKENS));
        thunkAPI.dispatch(doneStreaming({ id: chatId }));
      });
  },
);
