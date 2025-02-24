import { createReducer } from "@reduxjs/toolkit";
import {
  Chat,
  ChatThread,
  IntegrationMeta,
  ToolUse,
  LspChatMode,
  chatModeToLspMode,
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
  fixBrokenToolMessages,
  setIsNewChatSuggested,
  setIsNewChatSuggestionRejected,
  setThreadUsage,
} from "./actions";
import { formatChatResponse } from "./utils";
import {
  DEFAULT_MAX_NEW_TOKENS,
  isToolCallMessage,
  validateToolCall,
} from "../../../services/refact";
import { calculateUsageInputTokens } from "../../../utils/calculateUsageInputTokens";

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
    max_new_tokens: DEFAULT_MAX_NEW_TOKENS,
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

  builder.addCase(clearChatError, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.error = null;
  });

  builder.addCase(setChatModel, (state, action) => {
    state.thread.model = action.payload;
  });

  builder.addCase(setSystemPrompt, (state, action) => {
    state.system_prompt = action.payload;
  });

  builder.addCase(newChatAction, (state) => {
    const next = createInitialState({
      tool_use: state.tool_use,
      maybeMode: state.thread.mode,
    });
    next.cache = { ...state.cache };
    if (state.streaming) {
      next.cache[state.thread.id] = { ...state.thread, read: false };
    }
    next.thread.model = state.thread.model;
    next.system_prompt = state.system_prompt;
    next.automatic_patch = state.automatic_patch;
    next.checkpoints_enabled = state.checkpoints_enabled;
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

    state.streaming = true;
    state.waiting_for_response = false;
    state.thread.messages = messages;
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
    state.thread.read = true;
    state.prevent_send = false;
  });

  builder.addCase(setAutomaticPatch, (state, action) => {
    state.automatic_patch = action.payload;
  });

  builder.addCase(setIsNewChatSuggested, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.new_chat_suggested = {
      wasSuggested: action.payload.value,
    };
  });

  builder.addCase(setIsNewChatSuggestionRejected, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.new_chat_suggested = {
      ...state.thread.new_chat_suggested,
      wasRejectedByUser: action.payload.value,
    };
  });

  builder.addCase(setThreadUsage, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;

    const { usage } = action.payload;
    state.thread.usage = usage;

    const inputTokensAmount = calculateUsageInputTokens(usage, [
      "prompt_tokens",
      "cache_creation_input_tokens",
      "cache_read_input_tokens",
    ]);

    const maximumInputTokens = state.thread.currentMaximumContextTokens;

    if (maximumInputTokens && inputTokensAmount >= maximumInputTokens) {
      const { wasSuggested, wasRejectedByUser } =
        state.thread.new_chat_suggested;

      state.thread.new_chat_suggested = {
        wasSuggested: wasSuggested || !wasSuggested,
        wasRejectedByUser: wasRejectedByUser
          ? !wasRejectedByUser
          : wasRejectedByUser,
      };
    }
  });

  builder.addCase(setEnabledCheckpoints, (state, action) => {
    state.checkpoints_enabled = action.payload;
  });

  builder.addCase(setLastUserMessageId, (state, action) => {
    if (state.thread.id !== action.payload.chatId) return state;
    state.thread.last_user_message_id = action.payload.messageId;
  });

  builder.addCase(chatAskedQuestion, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.send_immediately = false;
    state.waiting_for_response = true;
    state.streaming = true;
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
    state.thread.tool_use = state.thread.tool_use ?? state.tool_use;
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

  builder.addCase(setMaxNewTokens, (state, action) => {
    state.max_new_tokens = action.payload;
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
});
