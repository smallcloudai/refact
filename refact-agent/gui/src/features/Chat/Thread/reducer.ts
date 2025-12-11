import { createReducer, Draft } from "@reduxjs/toolkit";
import {
  Chat,
  ChatThread,
  IntegrationMeta,
  ToolUse,
  LspChatMode,
  chatModeToLspMode,
  isLspChatMode,
} from "./types";
import { v4 as uuidv4 } from "uuid";
import { chatResponse, chatAskedQuestion } from ".";
import {
  setToolUse,
  enableSend,
  clearChatError,
  setChatModel,
  setSystemPrompt,
  newChatAction,
  backUpMessages,
  chatError,
  doneStreaming,
  removeChatFromCache,
  restoreChat,
  setPreventSend,
  saveTitle,
  newIntegrationChat,
  setSendImmediately,
  setChatMode,
  setIntegrationData,
  setIsWaitingForResponse,
  setMaxNewTokens,
  setAutomaticPatch,
  setLastUserMessageId,
  setEnabledCheckpoints,
  setBoostReasoning,
  fixBrokenToolMessages,
  setIsNewChatSuggested,
  setIsNewChatSuggestionRejected,
  upsertToolCall,
  setIncreaseMaxTokens,
  setAreFollowUpsEnabled,
  setIsTitleGenerationEnabled,
  setIncludeProjectInfo,
  setContextTokensCap,
  setUseCompression,
} from "./actions";
import { formatChatResponse, postProcessMessagesAfterStreaming } from "./utils";
import {
  ChatMessages,
  commandsApi,
  isAssistantMessage,
  isDiffMessage,
  isMultiModalToolResult,
  isToolCallMessage,
  isToolMessage,
  isUserMessage,
  isUserResponse,
  ToolCall,
  ToolMessage,
  UserMessage,
  validateToolCall,
} from "../../../services/refact";
import { capsApi } from "../../../services/refact";

const createChatThread = (
  tool_use: ToolUse,
  integration?: IntegrationMeta | null,
  mode?: LspChatMode,
): ChatThread => {
  const chat: ChatThread = {
    id: uuidv4(),
    messages: [],
    title: "",
    model: "",
    last_user_message_id: "",
    tool_use,
    integration,
    mode,
    new_chat_suggested: {
      wasSuggested: false,
    },
    boost_reasoning: false,
    automatic_patch: false,
    increase_max_tokens: false,
    include_project_info: true,
    context_tokens_cap: undefined,
  };
  return chat;
};

type createInitialStateArgs = {
  tool_use?: ToolUse;
  integration?: IntegrationMeta | null;
  maybeMode?: LspChatMode;
};

const getThreadMode = ({
  tool_use,
  integration,
  maybeMode,
}: createInitialStateArgs) => {
  if (integration) {
    return "CONFIGURE";
  }
  if (maybeMode) {
    return maybeMode === "CONFIGURE" ? "AGENT" : maybeMode;
  }

  return chatModeToLspMode({ toolUse: tool_use });
};

const createInitialState = ({
  tool_use = "agent",
  integration,
  maybeMode,
}: createInitialStateArgs): Chat => {
  const mode = getThreadMode({ tool_use, integration, maybeMode });

  return {
    streaming: false,
    thread: createChatThread(tool_use, integration, mode),
    error: null,
    prevent_send: false,
    waiting_for_response: false,
    cache: {},
    system_prompt: {},
    tool_use,
    checkpoints_enabled: true,
    send_immediately: false,
  };
};

const initialState = createInitialState({});

export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(setToolUse, (state, action) => {
    state.thread.tool_use = action.payload;
    state.tool_use = action.payload;
    state.thread.mode = chatModeToLspMode({ toolUse: action.payload });
  });

  builder.addCase(setPreventSend, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.prevent_send = true;
  });

  builder.addCase(enableSend, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.prevent_send = false;
  });

  builder.addCase(setAreFollowUpsEnabled, (state, action) => {
    state.follow_ups_enabled = action.payload;
  });

  builder.addCase(setIsTitleGenerationEnabled, (state, action) => {
    state.title_generation_enabled = action.payload;
  });

  builder.addCase(setUseCompression, (state, action) => {
    state.use_compression = action.payload;
  });

  builder.addCase(clearChatError, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.error = null;
  });

  builder.addCase(setChatModel, (state, action) => {
    state.thread.model = action.payload;
    state.thread.model = action.payload;
  });

  builder.addCase(setSystemPrompt, (state, action) => {
    state.system_prompt = action.payload;
  });

  builder.addCase(newChatAction, (state, action) => {
    const next = createInitialState({
      tool_use: state.tool_use,
      maybeMode: state.thread.mode,
    });
    next.cache = { ...state.cache };
    if (state.streaming || state.waiting_for_response) {
      next.cache[state.thread.id] = { ...state.thread, read: false };
    }
    next.thread.model = state.thread.model;
    next.system_prompt = state.system_prompt;
    next.checkpoints_enabled = state.checkpoints_enabled;
    next.follow_ups_enabled = state.follow_ups_enabled;
    next.title_generation_enabled = state.title_generation_enabled;
    next.use_compression = state.use_compression;
    next.thread.boost_reasoning = state.thread.boost_reasoning;
    // next.thread.automatic_patch = state.thread.automatic_patch;
    if (action.payload?.messages) {
      next.thread.messages = action.payload.messages;
    }
    return next;
  });

  builder.addCase(chatResponse, (state, action) => {
    if (
      action.payload.id !== state.thread.id &&
      !(action.payload.id in state.cache)
    ) {
      return state;
    }

    if (action.payload.id in state.cache) {
      const thread = state.cache[action.payload.id];
      // TODO: this might not be needed any more, because we can mutate the last message.
      const messages = formatChatResponse(thread.messages, action.payload);
      thread.messages = messages;
      return state;
    }

    const messages = formatChatResponse(state.thread.messages, action.payload);

    state.thread.messages = messages;
    state.streaming = true;
    state.waiting_for_response = false;

    if (
      isUserResponse(action.payload) &&
      action.payload.compression_strength &&
      action.payload.compression_strength !== "absent"
    ) {
      state.thread.new_chat_suggested = {
        wasRejectedByUser: false,
        wasSuggested: true,
      };
    }
  });

  builder.addCase(backUpMessages, (state, action) => {
    // TODO: should it also save to history?
    state.error = null;
    // state.previous_message_length = state.thread.messages.length;
    state.thread.messages = action.payload.messages;
  });

  builder.addCase(chatError, (state, action) => {
    state.streaming = false;
    state.prevent_send = true;
    state.waiting_for_response = false;
    state.error = action.payload.message;
  });

  builder.addCase(doneStreaming, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.streaming = false;
    state.waiting_for_response = false;
    state.thread.read = true;
    state.thread.messages = postProcessMessagesAfterStreaming(
      state.thread.messages,
    );
  });

  builder.addCase(setAutomaticPatch, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.automatic_patch = action.payload.value;
  });

  builder.addCase(setIsNewChatSuggested, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.new_chat_suggested = {
      wasSuggested: action.payload.value,
    };
  });

  builder.addCase(setIsNewChatSuggestionRejected, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.prevent_send = false;
    state.thread.new_chat_suggested = {
      ...state.thread.new_chat_suggested,
      wasRejectedByUser: action.payload.value,
    };
  });

  builder.addCase(setEnabledCheckpoints, (state, action) => {
    state.checkpoints_enabled = action.payload;
  });

  builder.addCase(setBoostReasoning, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.boost_reasoning = action.payload.value;
  });

  builder.addCase(setLastUserMessageId, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.last_user_message_id = action.payload.messageId;
  });

  builder.addCase(chatAskedQuestion, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.send_immediately = false;
    state.waiting_for_response = true;
    state.thread.read = false;
    state.prevent_send = false;
  });

  builder.addCase(removeChatFromCache, (state, action) => {
    if (!(action.payload.id in state.cache)) return state;

    const cache = Object.entries(state.cache).reduce<
      Record<string, ChatThread>
    >((acc, cur) => {
      if (cur[0] === action.payload.id) return acc;
      return { ...acc, [cur[0]]: cur[1] };
    }, {});
    state.cache = cache;
  });

  builder.addCase(restoreChat, (state, action) => {
    if (state.thread.id === action.payload.id) return state;
    const mostUptoDateThread =
      action.payload.id in state.cache
        ? { ...state.cache[action.payload.id] }
        : { ...action.payload, read: true };

    state.error = null;
    state.waiting_for_response = false;

    if (state.streaming) {
      state.cache[state.thread.id] = { ...state.thread, read: false };
    }
    if (action.payload.id in state.cache) {
      const { [action.payload.id]: _, ...rest } = state.cache;
      state.cache = rest;
      state.streaming = true;
    } else {
      state.streaming = false;
    }
    state.prevent_send = true;
    state.thread = {
      new_chat_suggested: { wasSuggested: false },
      ...mostUptoDateThread,
    };
    state.thread.messages = postProcessMessagesAfterStreaming(
      state.thread.messages,
    );
    state.thread.tool_use = state.thread.tool_use ?? state.tool_use;
    if (action.payload.mode && !isLspChatMode(action.payload.mode)) {
      state.thread.mode = "AGENT";
    }

    const lastUserMessage = action.payload.messages.reduce<UserMessage | null>(
      (acc, cur) => {
        if (isUserMessage(cur)) return cur;
        return acc;
      },
      null,
    );

    if (
      lastUserMessage?.compression_strength &&
      lastUserMessage.compression_strength !== "absent"
    ) {
      state.thread.new_chat_suggested = {
        wasRejectedByUser: false,
        wasSuggested: true,
      };
    }
  });

  // New builder to save chat title within the current thread and not only inside of a history thread
  builder.addCase(saveTitle, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.thread.title = action.payload.title;
    state.thread.isTitleGenerated = action.payload.isTitleGenerated;
  });

  builder.addCase(newIntegrationChat, (state, action) => {
    // TODO: find out about tool use
    // TODO: should be CONFIGURE ?
    const next = createInitialState({
      tool_use: "agent",
      integration: action.payload.integration,
      maybeMode: "CONFIGURE",
    });
    next.thread.last_user_message_id = action.payload.request_attempt_id;
    next.thread.integration = action.payload.integration;
    next.thread.messages = action.payload.messages;

    next.thread.model = state.thread.model;
    next.system_prompt = state.system_prompt;
    next.cache = { ...state.cache };
    if (state.streaming) {
      next.cache[state.thread.id] = { ...state.thread, read: false };
    }
    return next;
  });

  builder.addCase(setSendImmediately, (state, action) => {
    state.send_immediately = action.payload;
  });

  builder.addCase(setChatMode, (state, action) => {
    state.thread.mode = action.payload;
  });

  builder.addCase(setIntegrationData, (state, action) => {
    state.thread.integration = action.payload;
  });

  builder.addCase(setIsWaitingForResponse, (state, action) => {
    state.waiting_for_response = action.payload;
  });

  // TBD: should be safe to remove?
  builder.addCase(setMaxNewTokens, (state, action) => {
    state.thread.currentMaximumContextTokens = action.payload;
    // Also adjust context_tokens_cap if it exceeds the new max
    if (
      state.thread.context_tokens_cap === undefined ||
      state.thread.context_tokens_cap > action.payload
    ) {
      state.thread.context_tokens_cap = action.payload;
    }
  });

  builder.addCase(fixBrokenToolMessages, (state, action) => {
    if (action.payload.id !== state.thread.id) return state;
    if (state.thread.messages.length === 0) return state;
    const lastMessage = state.thread.messages[state.thread.messages.length - 1];
    if (!isToolCallMessage(lastMessage)) return state;
    if (lastMessage.tool_calls.every(validateToolCall)) return state;
    const validToolCalls = lastMessage.tool_calls.filter(validateToolCall);
    const messages = state.thread.messages.slice(0, -1);
    const newMessage = { ...lastMessage, tool_calls: validToolCalls };
    state.thread.messages = [...messages, newMessage];
  });

  builder.addCase(upsertToolCall, (state, action) => {
    // if (action.payload.toolCallId !== state.thread.id && !(action.payload.chatId in state.cache)) return state;
    if (action.payload.chatId === state.thread.id) {
      maybeAppendToolCallResultFromIdeToMessages(
        state.thread.messages,
        action.payload.toolCallId,
        action.payload.accepted,
      );
    } else if (action.payload.chatId in state.cache) {
      const thread = state.cache[action.payload.chatId];
      maybeAppendToolCallResultFromIdeToMessages(
        thread.messages,
        action.payload.toolCallId,
        action.payload.accepted,
        action.payload.replaceOnly,
      );
    }
  });

  builder.addCase(setIncreaseMaxTokens, (state, action) => {
    state.thread.increase_max_tokens = action.payload;
  });

  builder.addCase(setIncludeProjectInfo, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.include_project_info = action.payload.value;
  });

  builder.addCase(setContextTokensCap, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.context_tokens_cap = action.payload.value;
  });

  builder.addMatcher(
    capsApi.endpoints.getCaps.matchFulfilled,
    (state, action) => {
      const defaultModel = action.payload.chat_default_model;

      const model = state.thread.model || defaultModel;
      if (!(model in action.payload.chat_models)) return;

      const currentModelMaximumContextTokens =
        action.payload.chat_models[model].n_ctx;

      state.thread.currentMaximumContextTokens =
        currentModelMaximumContextTokens;

      if (
        state.thread.context_tokens_cap === undefined ||
        state.thread.context_tokens_cap > currentModelMaximumContextTokens
      ) {
        state.thread.context_tokens_cap = currentModelMaximumContextTokens;
      }
    },
  );

  builder.addMatcher(
    commandsApi.endpoints.getCommandPreview.matchFulfilled,
    (state, action) => {
      state.thread.currentMaximumContextTokens = action.payload.number_context;
      state.thread.currentMessageContextTokens = action.payload.current_context; // assuming that this number is amount of tokens per current message
    },
  );
});

export function maybeAppendToolCallResultFromIdeToMessages(
  messages: Draft<ChatMessages>,
  toolCallId: string,
  accepted: boolean | "indeterminate",
  replaceOnly = false,
) {
  const hasDiff = messages.find(
    (d) => isDiffMessage(d) && d.tool_call_id === toolCallId,
  );
  if (hasDiff) return;

  const maybeToolResult = messages.find(
    (d) => isToolMessage(d) && d.content.tool_call_id === toolCallId,
  );

  const toolCalls = messages.reduce<ToolCall[]>((acc, message) => {
    if (!isAssistantMessage(message)) return acc;
    if (!message.tool_calls) return acc;
    return acc.concat(message.tool_calls);
  }, []);

  const maybeToolCall = toolCalls.find(
    (toolCall) => toolCall.id === toolCallId,
  );

  const message = messageForToolCall(accepted, maybeToolCall);

  if (replaceOnly && !maybeToolResult) return;

  if (
    maybeToolResult &&
    isToolMessage(maybeToolResult) &&
    typeof maybeToolResult.content.content === "string"
  ) {
    maybeToolResult.content.content = message;
    return;
  } else if (
    maybeToolResult &&
    isToolMessage(maybeToolResult) &&
    isMultiModalToolResult(maybeToolResult.content)
  ) {
    maybeToolResult.content.content.push({
      m_type: "text",
      m_content: message,
    });
    return;
  }

  const assistantMessageIndex = messages.findIndex((message) => {
    if (!isAssistantMessage(message)) return false;
    return message.tool_calls?.find((toolCall) => toolCall.id === toolCallId);
  });

  if (assistantMessageIndex === -1) return;
  const toolMessage: ToolMessage = {
    role: "tool",
    content: {
      content: message,
      tool_call_id: toolCallId,
      // assuming, that tool_failed is always false at this point
      tool_failed: false,
    },
  };

  messages.splice(assistantMessageIndex + 1, 0, toolMessage);
}

function messageForToolCall(
  accepted: boolean | "indeterminate",
  toolCall?: ToolCall,
) {
  if (accepted === false && toolCall?.function.name) {
    return `Whoops the user didn't like the command ${toolCall.function.name}. Stop and ask for correction from the user.`;
  }
  if (accepted === false) return "The user rejected the changes.";
  if (accepted === true) return "The user accepted the changes.";
  return "The user may have made modifications to changes.";
}
