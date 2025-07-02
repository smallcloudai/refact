import { createReducer, Draft } from "@reduxjs/toolkit";
import {
  Chat,
  ChatThread,
  IntegrationMeta,
  ToolUse,
  LspChatMode,
  chatModeToLspMode,
} from "./types";
import { v4 as uuidv4 } from "uuid";
import {
  setToolUse,
  newChatAction,
  chatError,
  doneStreaming,
  setPreventSend,
  newIntegrationChat,
  setIntegrationData,
  setMaxNewTokens,
  setEnabledCheckpoints,
  fixBrokenToolMessages,
  setIsTitleGenerationEnabled,
  chatResponse,
} from "./actions";
import { formatChatResponse } from "./utils";
import {
  ChatMessages,
  commandsApi,
  isAssistantMessage,
  isDiffMessage,
  // isMultiModalToolResult,
  isToolCallMessage,
  isToolMessage,
  isUserResponse,
  ToolCall,
  ToolMessage,
  validateToolCall,
} from "../../../services/refact";

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

  builder.addCase(setIsTitleGenerationEnabled, (state, action) => {
    state.title_generation_enabled = action.payload;
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

    next.checkpoints_enabled = state.checkpoints_enabled;
    next.follow_ups_enabled = state.follow_ups_enabled;
    next.title_generation_enabled = state.title_generation_enabled;
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
  });

  builder.addCase(setEnabledCheckpoints, (state, action) => {
    state.checkpoints_enabled = action.payload;
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
    next.cache = { ...state.cache };
    if (state.streaming) {
      next.cache[state.thread.id] = { ...state.thread, read: false };
    }
    return next;
  });

  builder.addCase(setIntegrationData, (state, action) => {
    state.thread.integration = action.payload;
  });

  // TBD: should be safe to remove?
  builder.addCase(setMaxNewTokens, (state, action) => {
    state.thread.currentMaximumContextTokens = action.payload;
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
    (d) => isToolMessage(d) && d.ftm_call_id === toolCallId,
  );

  const toolCalls = messages.reduce<ToolCall[]>((acc, message) => {
    if (!isAssistantMessage(message)) return acc;
    if (!message.ftm_tool_calls) return acc;
    return acc.concat(message.ftm_tool_calls);
  }, []);

  const maybeToolCall = toolCalls.find(
    (toolCall) => toolCall.id === toolCallId,
  );

  const message = messageForToolCall(accepted, maybeToolCall);

  if (replaceOnly && !maybeToolResult) return;

  if (
    maybeToolResult &&
    isToolMessage(maybeToolResult) &&
    typeof maybeToolResult.ftm_content === "string"
  ) {
    maybeToolResult.ftm_content = message;
    return;
  } else if (
    maybeToolResult &&
    isToolMessage(maybeToolResult) &&
    Array.isArray(maybeToolResult.ftm_content)
  ) {
    maybeToolResult.ftm_content.push({
      m_type: "text",
      m_content: message,
    });
    return;
  }

  const assistantMessageIndex = messages.findIndex((message) => {
    if (!isAssistantMessage(message)) return false;
    return message.ftm_tool_calls?.find(
      (toolCall) => toolCall.id === toolCallId,
    );
  });

  if (assistantMessageIndex === -1) return;
  const toolMessage: ToolMessage = {
    ftm_role: "tool",
    ftm_content: message,
    ftm_call_id: toolCallId,
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
