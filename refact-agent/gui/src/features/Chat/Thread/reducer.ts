import { createReducer, Draft } from "@reduxjs/toolkit";
import {
  Chat,
  ChatThread,
  ChatThreadRuntime,
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
  setIncludeProjectInfo,
  setContextTokensCap,
  setUseCompression,
  enqueueUserMessage,
  dequeueUserMessage,
  clearQueuedMessages,
  closeThread,
  switchToThread,
  updateOpenThread,
  setThreadPauseReasons,
  clearThreadPauseReasons,
  setThreadConfirmationStatus,
  addThreadImage,
  removeThreadImageByIndex,
  resetThreadImages,
  chatAskQuestionThunk,
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
  validateToolCall,
} from "../../../services/refact";
import { capsApi } from "../../../services/refact";

const createChatThread = (
  tool_use: ToolUse,
  integration?: IntegrationMeta | null,
  mode?: LspChatMode,
): ChatThread => {
  return {
    id: uuidv4(),
    messages: [],
    title: "",
    model: "",
    last_user_message_id: "",
    tool_use,
    integration,
    mode,
    new_chat_suggested: { wasSuggested: false },
    boost_reasoning: false,
    automatic_patch: false,
    increase_max_tokens: false,
    include_project_info: true,
    context_tokens_cap: undefined,
  };
};

const createThreadRuntime = (
  tool_use: ToolUse,
  integration?: IntegrationMeta | null,
  mode?: LspChatMode,
): ChatThreadRuntime => {
  return {
    thread: createChatThread(tool_use, integration, mode),
    streaming: false,
    waiting_for_response: false,
    prevent_send: false,
    error: null,
    queued_messages: [],
    send_immediately: false,
    attached_images: [],
    confirmation: {
      pause: false,
      pause_reasons: [],
      status: {
        wasInteracted: false,
        confirmationStatus: true,
      },
    },
  };
};

const getThreadMode = ({
  tool_use,
  integration,
  maybeMode,
}: {
  tool_use?: ToolUse;
  integration?: IntegrationMeta | null;
  maybeMode?: LspChatMode;
}) => {
  if (integration) return "CONFIGURE";
  if (maybeMode) return maybeMode === "CONFIGURE" ? "AGENT" : maybeMode;
  return chatModeToLspMode({ toolUse: tool_use });
};

const createInitialState = (): Chat => {
  return {
    current_thread_id: "",
    open_thread_ids: [],
    threads: {},
    system_prompt: {},
    tool_use: "agent",
    checkpoints_enabled: true,
    follow_ups_enabled: undefined,
    use_compression: undefined,
  };
};

const initialState = createInitialState();

const getRuntime = (state: Draft<Chat>, chatId: string): Draft<ChatThreadRuntime> | null => {
  return state.threads[chatId] ?? null;
};

const getCurrentRuntime = (state: Draft<Chat>): Draft<ChatThreadRuntime> | null => {
  return getRuntime(state, state.current_thread_id);
};



export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(setToolUse, (state, action) => {
    state.tool_use = action.payload;
    const rt = getCurrentRuntime(state);
    if (rt) {
      rt.thread.tool_use = action.payload;
      rt.thread.mode = chatModeToLspMode({ toolUse: action.payload });
    }
  });

  builder.addCase(setPreventSend, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) rt.prevent_send = true;
  });

  builder.addCase(enableSend, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) rt.prevent_send = false;
  });

  builder.addCase(setAreFollowUpsEnabled, (state, action) => {
    state.follow_ups_enabled = action.payload;
  });

  builder.addCase(setUseCompression, (state, action) => {
    state.use_compression = action.payload;
  });

  builder.addCase(clearChatError, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) rt.error = null;
  });

  builder.addCase(setChatModel, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.thread.model = action.payload;
  });

  builder.addCase(setSystemPrompt, (state, action) => {
    state.system_prompt = action.payload;
  });

  builder.addCase(newChatAction, (state, action) => {
    const currentRt = getCurrentRuntime(state);
    const mode = getThreadMode({ tool_use: state.tool_use, maybeMode: currentRt?.thread.mode });
    const newRuntime = createThreadRuntime(state.tool_use, null, mode);

    if (currentRt) {
      newRuntime.thread.model = currentRt.thread.model;
      newRuntime.thread.boost_reasoning = currentRt.thread.boost_reasoning;
    }

    if (action.payload?.messages) {
      newRuntime.thread.messages = action.payload.messages;
    }

    const newId = newRuntime.thread.id;
    state.threads[newId] = newRuntime;
    state.open_thread_ids.push(newId);
    state.current_thread_id = newId;
  });

  builder.addCase(chatResponse, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (!rt) return;

    const messages = formatChatResponse(rt.thread.messages, action.payload);
    rt.thread.messages = messages;
    rt.streaming = true;
    rt.waiting_for_response = false;

    if (
      isUserResponse(action.payload) &&
      action.payload.compression_strength &&
      action.payload.compression_strength !== "absent"
    ) {
      rt.thread.new_chat_suggested = {
        wasRejectedByUser: false,
        wasSuggested: true,
      };
    }
  });

  builder.addCase(backUpMessages, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.error = null;
      rt.thread.messages = action.payload.messages;
    }
  });

  builder.addCase(chatError, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.streaming = false;
      rt.prevent_send = true;
      rt.waiting_for_response = false;
      rt.error = action.payload.message;
    }
  });

  builder.addCase(doneStreaming, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.streaming = false;
      rt.waiting_for_response = false;
      rt.thread.read = action.payload.id === state.current_thread_id;
      rt.thread.messages = postProcessMessagesAfterStreaming(rt.thread.messages);
    }
  });

  builder.addCase(setAutomaticPatch, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.automatic_patch = action.payload.value;
  });

  builder.addCase(setIsNewChatSuggested, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.new_chat_suggested = { wasSuggested: action.payload.value };
  });

  builder.addCase(setIsNewChatSuggestionRejected, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.prevent_send = false;
      rt.thread.new_chat_suggested = {
        ...rt.thread.new_chat_suggested,
        wasRejectedByUser: action.payload.value,
      };
    }
  });

  builder.addCase(setEnabledCheckpoints, (state, action) => {
    state.checkpoints_enabled = action.payload;
  });

  builder.addCase(setBoostReasoning, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.boost_reasoning = action.payload.value;
  });

  builder.addCase(setLastUserMessageId, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.last_user_message_id = action.payload.messageId;
  });

  builder.addCase(chatAskedQuestion, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.send_immediately = false;
      rt.waiting_for_response = true;
      rt.thread.read = false;
      rt.prevent_send = false;
    }
  });

  builder.addCase(removeChatFromCache, (state, action) => {
    const id = action.payload.id;
    const rt = state.threads[id];
    if (rt && !rt.streaming && !rt.confirmation.pause) {
      delete state.threads[id];
      state.open_thread_ids = state.open_thread_ids.filter((tid) => tid !== id);
    }
  });

  builder.addCase(closeThread, (state, action) => {
    const id = action.payload.id;
    const force = action.payload.force ?? false;
    state.open_thread_ids = state.open_thread_ids.filter((tid) => tid !== id);
    const rt = state.threads[id];
    if (rt && (force || (!rt.streaming && !rt.waiting_for_response && !rt.confirmation.pause))) {
      delete state.threads[id];
    }
    if (state.current_thread_id === id) {
      state.current_thread_id = state.open_thread_ids[0] ?? "";
    }
  });

  builder.addCase(restoreChat, (state, action) => {
    const existingRt = getRuntime(state, action.payload.id);
    if (existingRt) {
      // Runtime exists (possibly running in background) - re-add to open tabs if needed
      if (!state.open_thread_ids.includes(action.payload.id)) {
        state.open_thread_ids.push(action.payload.id);
      }
      state.current_thread_id = action.payload.id;
      existingRt.thread.read = true;
      return;
    }

    const mode = action.payload.mode && isLspChatMode(action.payload.mode)
      ? action.payload.mode
      : "AGENT";
    const newRuntime: ChatThreadRuntime = {
      thread: {
        new_chat_suggested: { wasSuggested: false },
        ...action.payload,
        mode,
        tool_use: action.payload.tool_use ?? state.tool_use,
        read: true,
      },
      streaming: false,
      waiting_for_response: false,
      prevent_send: false,
      error: null,
      queued_messages: [],
      send_immediately: false,
      attached_images: [],
      confirmation: {
        pause: false,
        pause_reasons: [],
        status: {
          wasInteracted: false,
          confirmationStatus: true,
        },
      },
    };
    newRuntime.thread.messages = postProcessMessagesAfterStreaming(
      newRuntime.thread.messages,
    );

    const lastUserMessage = action.payload.messages.reduce<import("../../../services/refact/types").UserMessage | null>(
      (acc, cur) => (isUserMessage(cur) ? cur : acc),
      null,
    );
    if (
      lastUserMessage?.compression_strength &&
      lastUserMessage.compression_strength !== "absent"
    ) {
      newRuntime.thread.new_chat_suggested = {
        wasRejectedByUser: false,
        wasSuggested: true,
      };
    }

    state.threads[action.payload.id] = newRuntime;
    if (!state.open_thread_ids.includes(action.payload.id)) {
      state.open_thread_ids.push(action.payload.id);
    }
    state.current_thread_id = action.payload.id;
  });

  builder.addCase(switchToThread, (state, action) => {
    const id = action.payload.id;
    const existingRt = getRuntime(state, id);
    if (existingRt) {
      if (!state.open_thread_ids.includes(id)) {
        state.open_thread_ids.push(id);
      }
      state.current_thread_id = id;
      existingRt.thread.read = true;
    }
  });

  // Update an already-open thread with fresh data from backend (used by subscription)
  // Only updates if the thread is not currently streaming, waiting, or has an error
  builder.addCase(updateOpenThread, (state, action) => {
    const existingRt = getRuntime(state, action.payload.id);
    // Don't update if:
    // - Thread doesn't exist
    // - Thread is actively streaming
    // - Thread is waiting for response
    // - Thread has an error (avoid overwriting with stale data)
    // - Thread is the current active thread (user is viewing it)
    if (
      existingRt &&
      !existingRt.streaming &&
      !existingRt.waiting_for_response &&
      !existingRt.error &&
      action.payload.id !== state.current_thread_id
    ) {
      existingRt.thread = {
        ...existingRt.thread,
        ...action.payload.thread,
      };
    }
  });

  builder.addCase(saveTitle, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.thread.title = action.payload.title;
      rt.thread.isTitleGenerated = action.payload.isTitleGenerated;
    }
  });

  builder.addCase(newIntegrationChat, (state, action) => {
    const currentRt = getCurrentRuntime(state);
    const newRuntime = createThreadRuntime("agent", action.payload.integration, "CONFIGURE");
    newRuntime.thread.last_user_message_id = action.payload.request_attempt_id;
    newRuntime.thread.messages = action.payload.messages;
    if (currentRt) {
      newRuntime.thread.model = currentRt.thread.model;
    }

    const newId = newRuntime.thread.id;
    state.threads[newId] = newRuntime;
    state.open_thread_ids.push(newId);
    state.current_thread_id = newId;
  });

  builder.addCase(setSendImmediately, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.send_immediately = action.payload;
  });

  builder.addCase(enqueueUserMessage, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (!rt) return;
    const { priority, ...rest } = action.payload;
    const messagePayload = { ...rest, priority };
    if (priority) {
      const insertAt = rt.queued_messages.findIndex((m) => !m.priority);
      if (insertAt === -1) {
        rt.queued_messages.push(messagePayload);
      } else {
        rt.queued_messages.splice(insertAt, 0, messagePayload);
      }
    } else {
      rt.queued_messages.push(messagePayload);
    }
  });

  builder.addCase(dequeueUserMessage, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) {
      rt.queued_messages = rt.queued_messages.filter(
        (q) => q.id !== action.payload.queuedId,
      );
    }
  });

  builder.addCase(clearQueuedMessages, (state) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.queued_messages = [];
  });

  builder.addCase(setChatMode, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.thread.mode = action.payload;
  });

  builder.addCase(setIntegrationData, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.thread.integration = action.payload;
  });

  builder.addCase(setIsWaitingForResponse, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) rt.waiting_for_response = action.payload.value;
  });

  builder.addCase(setMaxNewTokens, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) {
      rt.thread.currentMaximumContextTokens = action.payload;
      if (
        rt.thread.context_tokens_cap === undefined ||
        rt.thread.context_tokens_cap > action.payload
      ) {
        rt.thread.context_tokens_cap = action.payload;
      }
    }
  });

  builder.addCase(fixBrokenToolMessages, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (!rt || rt.thread.messages.length === 0) return;
    const lastMessage = rt.thread.messages[rt.thread.messages.length - 1];
    if (!isToolCallMessage(lastMessage)) return;
    if (lastMessage.tool_calls.every(validateToolCall)) return;
    const validToolCalls = lastMessage.tool_calls.filter(validateToolCall);
    const messages = rt.thread.messages.slice(0, -1);
    const newMessage = { ...lastMessage, tool_calls: validToolCalls };
    rt.thread.messages = [...messages, newMessage];
  });

  builder.addCase(upsertToolCall, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      maybeAppendToolCallResultFromIdeToMessages(
        rt.thread.messages,
        action.payload.toolCallId,
        action.payload.accepted,
        action.payload.replaceOnly,
      );
    }
  });

  builder.addCase(setIncreaseMaxTokens, (state, action) => {
    const rt = getCurrentRuntime(state);
    if (rt) rt.thread.increase_max_tokens = action.payload;
  });

  builder.addCase(setIncludeProjectInfo, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.include_project_info = action.payload.value;
  });

  builder.addCase(setContextTokensCap, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.context_tokens_cap = action.payload.value;
  });

  builder.addCase(setThreadPauseReasons, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.confirmation.pause = true;
      rt.confirmation.pause_reasons = action.payload.pauseReasons;
      rt.confirmation.status.wasInteracted = false;
      rt.confirmation.status.confirmationStatus = false;
      rt.streaming = false;
      rt.waiting_for_response = false;
    }
  });

  builder.addCase(clearThreadPauseReasons, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.confirmation.pause = false;
      rt.confirmation.pause_reasons = [];
    }
  });

  builder.addCase(setThreadConfirmationStatus, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.confirmation.status.wasInteracted = action.payload.wasInteracted;
      rt.confirmation.status.confirmationStatus = action.payload.confirmationStatus;
    }
  });

  builder.addCase(addThreadImage, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt && rt.attached_images.length < 10) {
      rt.attached_images.push(action.payload.image);
    }
  });

  builder.addCase(removeThreadImageByIndex, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.attached_images = rt.attached_images.filter(
        (_, index) => index !== action.payload.index,
      );
    }
  });

  builder.addCase(resetThreadImages, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.attached_images = [];
    }
  });

  builder.addMatcher(
    capsApi.endpoints.getCaps.matchFulfilled,
    (state, action) => {
      const defaultModel = action.payload.chat_default_model;
      const rt = getCurrentRuntime(state);
      if (!rt) return;

      const model = rt.thread.model || defaultModel;
      if (!(model in action.payload.chat_models)) return;

      const currentModelMaximumContextTokens =
        action.payload.chat_models[model].n_ctx;

      rt.thread.currentMaximumContextTokens = currentModelMaximumContextTokens;

      if (
        rt.thread.context_tokens_cap === undefined ||
        rt.thread.context_tokens_cap > currentModelMaximumContextTokens
      ) {
        rt.thread.context_tokens_cap = currentModelMaximumContextTokens;
      }
    },
  );

  builder.addMatcher(
    commandsApi.endpoints.getCommandPreview.matchFulfilled,
    (state, action) => {
      const rt = getCurrentRuntime(state);
      if (rt) {
        rt.thread.currentMaximumContextTokens = action.payload.number_context;
        rt.thread.currentMessageContextTokens = action.payload.current_context;
      }
    },
  );

  // Handle rejected chat requests - set error state so spinner hides and SSE doesn't overwrite
  builder.addMatcher(
    chatAskQuestionThunk.rejected.match,
    (state, action) => {
      const chatId = action.meta.arg.chatId;
      const rt = getRuntime(state, chatId);
      if (rt && action.payload) {
        const payload = action.payload as { detail?: string };
        rt.error = payload.detail ?? "Unknown error";
        rt.prevent_send = true;
        rt.streaming = false;
        rt.waiting_for_response = false;
      }
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
