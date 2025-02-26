import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import {
  type PayloadWithIdAndTitle,
  type ChatThread,
  type PayloadWithId,
  type ToolUse,
  IntegrationMeta,
  LspChatMode,
  PayloadWithChatAndMessageId,
  PayloadWithChatAndBoolean,
  PayloadWithChatAndUsage,
  PayloadWithChatAndNumber,
  PayloadWithChatAndCurrentUsage,
} from "./types";
import {
  isAssistantDelta,
  isAssistantMessage,
  isCDInstructionMessage,
  isChatResponseChoice,
  // isChatGetTitleResponse,
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
import {
  DEFAULT_MAX_NEW_TOKENS,
  generateChatTitle,
  sendChat,
} from "../../../services/refact/chat";
import { ToolCommand, toolsApi } from "../../../services/refact/tools";
import { scanFoDuplicatesWith, takeFromEndWhile } from "../../../utils";
import { debugApp } from "../../../debugConfig";
import { ChatHistoryItem } from "../../History/historySlice";
import { ideToolCallResponse } from "../../../hooks/useEventBusForIDE";

export const newChatAction = createAction("chatThread/new");

export const newIntegrationChat = createAction<{
  integration: IntegrationMeta;
  messages: ChatMessages;
}>("chatThread/newIntegrationChat");

export const chatResponse = createAction<PayloadWithId & ChatResponse>(
  "chatThread/response",
);

export const chatTitleGenerationResponse = createAction<
  PayloadWithId & ChatResponse
>("chatTitleGeneration/response");

export const chatAskedQuestion = createAction<PayloadWithId>(
  "chatThread/askQuestion",
);

export const setLastUserMessageId = createAction<PayloadWithChatAndMessageId>(
  "chatThread/setLastUserMessageId",
);

export const setUsageTokensOnCommandPreview =
  createAction<PayloadWithChatAndCurrentUsage>(
    "chatThread/setUsageTokensOnCommandPreview",
  );

export const updateMaximumContextTokens =
  createAction<PayloadWithChatAndNumber>(
    "chatThread/updateMaximumContextTokens",
  );

export const setIsNewChatSuggested = createAction<PayloadWithChatAndBoolean>(
  "chatThread/setIsNewChatSuggested",
);

export const setIsNewChatCreationMandatory =
  createAction<PayloadWithChatAndBoolean>(
    "chatThread/setIsNewChatCreationMandatory",
  );

export const setThreadUsage = createAction<PayloadWithChatAndUsage>(
  "chatThread/setThreadUsage",
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
  state.chat.thread.model;

export const setSystemPrompt = createAction<SystemPrompts>(
  "chatThread/setSystemPrompt",
);

export const removeChatFromCache = createAction<PayloadWithId>(
  "chatThread/removeChatFromCache",
);

export const restoreChat = createAction<ChatHistoryItem>(
  "chatThread/restoreChat",
);

export const clearChatError = createAction<PayloadWithId>(
  "chatThread/clearError",
);

export const enableSend = createAction<PayloadWithId>("chatThread/enableSend");
export const setPreventSend = createAction<PayloadWithId>(
  "chatThread/preventSend",
);

export const setToolUse = createAction<ToolUse>("chatThread/setToolUse");

export const setAutomaticPatch = createAction<boolean>(
  "chat/setAutomaticPatch",
);

export const setEnabledCheckpoints = createAction<boolean>(
  "chat/setEnabledCheckpoints",
);

export const saveTitle = createAction<PayloadWithIdAndTitle>(
  "chatThread/saveTitle",
);

export const setSendImmediately = createAction<boolean>(
  "chatThread/setSendImmediately",
);

export const setChatMode = createAction<LspChatMode>("chatThread/setChatMode");

export const setIntegrationData = createAction<Partial<IntegrationMeta> | null>(
  "chatThread/setIntegrationData",
);

export const setIsWaitingForResponse = createAction<boolean>(
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

// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

export const chatGenerateTitleThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
  }
>("chatThread/generateTitle", ({ messages, chatId }, thunkAPI) => {
  const state = thunkAPI.getState();

  const messagesToSend = messages.filter(
    (msg) =>
      !isToolMessage(msg) && !isAssistantMessage(msg) && msg.content !== "",
  );
  // .map((msg) => {
  //   if (isAssistantMessage(msg)) {
  //     return {
  //       role: msg.role,
  //       content: msg.content,
  //     };
  //   }
  //   return msg;
  // });
  debugApp(`[DEBUG TITLE]: messagesToSend: `, messagesToSend);
  const messagesForLsp = formatMessagesForLsp([
    ...messagesToSend,
    {
      role: "user",
      content:
        "Generate a short 2-3 word title for the current chat that reflects the context of the user's query. The title should be specific, avoiding generic terms, and should relate to relevant files, symbols, or objects. If user message contains filename, please make sure that filename remains inside of a generated title. Please ensure the answer is strictly 2-3 words, not paragraphs of text.\nOutput should be STRICTLY 2-3 words, not explanation.",
      checkpoints: [],
    },
  ]);

  const chatResponseChunks: ChatResponse[] = [];

  return generateChatTitle({
    messages: messagesForLsp,
    model: state.chat.thread.model,
    stream: true,
    abortSignal: thunkAPI.signal,
    chatId,
    apiKey: state.config.apiKey,
    port: state.config.lspPort,
  })
    .then((response) => {
      if (!response.ok) {
        return Promise.reject(new Error(response.statusText));
      }
      const reader = response.body?.getReader();
      if (!reader) return;
      const onAbort = () => thunkAPI.dispatch(setPreventSend({ id: chatId }));
      const onChunk = (json: Record<string, unknown>) => {
        chatResponseChunks.push(json as ChatResponse);
      };
      return consumeStream(reader, thunkAPI.signal, onAbort, onChunk);
    })
    .catch((err: Error) => {
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
      thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
      return thunkAPI.rejectWithValue(err.message);
    })
    .finally(() => {
      const title = chatResponseChunks.reduce<string>((acc, chunk) => {
        if (isChatResponseChoice(chunk)) {
          if (isAssistantDelta(chunk.choices[0].delta)) {
            return acc + chunk.choices[0].delta.content;
          }
        }
        return acc;
      }, "");

      thunkAPI.dispatch(
        saveTitle({ id: chatId, title, isTitleGenerated: true }),
      );
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
    });
});

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

// export function chatModeToLspMode(mode?: ToolUse) {
//   if (mode === "agent") return "AGENT";
//   if (mode === "quick") return "NO_TOOLS";
//   return "EXPLORE";
// }

export const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    tools: ToolCommand[] | null;
    toolsConfirmed?: boolean;
    checkpointsEnabled?: boolean;
    mode?: LspChatMode; // used once for actions
    // TODO: make a separate function for this... and it'll need to be saved.
  }
>(
  "chatThread/sendChat",
  (
    { messages, chatId, tools, mode, toolsConfirmed, checkpointsEnabled },
    thunkAPI,
  ) => {
    const state = thunkAPI.getState();

    const thread =
      chatId in state.chat.cache
        ? state.chat.cache[chatId]
        : state.chat.thread.id === chatId
          ? state.chat.thread
          : null;

    // TODO: stops the stream.
    // const onlyDeterministicMessages =
    //   checkForToolLoop(messages) || !messages.some(isSystemMessage);

    const onlyDeterministicMessages = checkForToolLoop(messages);

    const messagesForLsp = formatMessagesForLsp(messages);
    const realMode = mode ?? thread?.mode;
    const maybeLastUserMessageId = thread?.last_user_message_id;

    return sendChat({
      messages: messagesForLsp,
      last_user_message_id: maybeLastUserMessageId,
      model: state.chat.thread.model,
      tools,
      stream: true,
      abortSignal: thunkAPI.signal,
      max_new_tokens: state.chat.max_new_tokens,
      chatId,
      apiKey: state.config.apiKey,
      port: state.config.lspPort,
      onlyDeterministicMessages,
      toolsConfirmed: toolsConfirmed,
      checkpointsEnabled,
      integration: thread?.integration,
      mode: realMode,
    })
      .then((response) => {
        if (!response.ok) {
          return Promise.reject(new Error(response.statusText));
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
      .catch((err: Error) => {
        // console.log("Catch called");
        thunkAPI.dispatch(doneStreaming({ id: chatId }));
        thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
        thunkAPI.dispatch(fixBrokenToolMessages({ id: chatId }));
        return thunkAPI.rejectWithValue(err.message);
      })
      .finally(() => {
        thunkAPI.dispatch(setMaxNewTokens(DEFAULT_MAX_NEW_TOKENS));
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
    const toolUse = state.chat.thread.tool_use;
    if (state.chat.thread.id !== chatId) return;
    if (
      state.chat.streaming ||
      state.chat.prevent_send ||
      state.chat.waiting_for_response
    ) {
      return;
    }
    const lastMessages = takeFromEndWhile(
      state.chat.thread.messages,
      (message) => !isUserMessage(message) && !isAssistantMessage(message),
    );

    const toolUseInThisSet = lastMessages.some(
      (message) =>
        isToolMessage(message) && message.content.tool_call_id === toolCallId,
    );

    if (!toolUseInThisSet) return;
    thunkApi.dispatch(setIsWaitingForResponse(true));
    // duplicate in sendChat
    let tools = await thunkApi
      .dispatch(toolsApi.endpoints.getTools.initiate(undefined))
      .unwrap();

    if (toolUse === "quick") {
      tools = [];
    } else if (toolUse === "explore") {
      tools = tools.filter((t) => !t.function.agentic);
    }
    tools = tools.map((t) => {
      const { agentic: _, ...remaining } = t.function;
      return { ...t, function: { ...remaining } };
    });

    return thunkApi.dispatch(
      chatAskQuestionThunk({
        messages: state.chat.thread.messages,
        tools,
        chatId,
        mode: state.chat.thread.mode,
        checkpointsEnabled: state.chat.checkpoints_enabled,
      }),
    );
  },
);
