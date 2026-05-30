import { createReducer, Draft } from "@reduxjs/toolkit";
import {
  Chat,
  ChatThread,
  ChatThreadRuntime,
  IntegrationMeta,
  ToolUse,
  ChatModeId,
  isToolUse,
  normalizeLegacyMode,
  isReasoningEffort,
} from "./types";
import { v4 as uuidv4 } from "uuid";
import { getLastThreadParams } from "../../../utils/threadStorage";
import {
  setToolUse,
  setThreadMode,
  setThreadWorktree,
  enableSend,
  clearChatError,
  setChatModel,
  setSystemPrompt,
  newChatAction,
  createChatWithId,
  backUpMessages,
  removeChatFromCache,
  restoreChat,
  setPreventSend,
  saveTitle,
  newIntegrationChat,
  setSendImmediately,
  setChatMode,
  setIntegrationData,
  setIsWaitingForResponse,
  markThreadSseError,
  setMaxNewTokens,
  setAutoApproveEditingTools,
  setAutoApproveDangerousCommands,
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
  setReasoningEffort,
  setThinkingBudget,
  setTemperature,
  setFrequencyPenalty,
  setMaxTokens,
  setParallelToolCalls,
  closeThread,
  switchToThread,
  updateOpenThread,
  updateChatRuntimeFromSessionState,
  setThreadPauseReasons,
  clearThreadPauseReasons,
  setThreadConfirmationStatus,
  addThreadImage,
  removeThreadImageByIndex,
  resetThreadImages,
  addThreadTextFile,
  removeThreadTextFileByIndex,
  resetThreadTextFiles,
  applyChatEvent,
  requestSseRefresh,
  clearSseRefreshRequest,
  setTaskWidgetExpanded,
  setAutoEnrichmentEnabled,
  setAutoCompactEnabled,
  markMemoryEnrichmentUserTouched,
  setManualPreviewItems,
  removeManualPreviewItem,
  clearManualPreviewItems,
  openBuddyChat,
  newBuddyChatAction,
  hydratePersistedChatTabs,
} from "./actions";
import { applyDeltaOps } from "../../../services/refact/chatSubscription";
import type { WorktreeMeta } from "../../../services/refact/worktrees";
import { loadPersistedChatTabs } from "../../../utils/chatUiPersistence";
import {
  AssistantMessage,
  ChatMessages,
  commandsApi,
  isAssistantMessage,
  isDiffMessage,
  isToolCallMessage,
  isToolMessage,
  ToolCall,
  ToolConfirmationPauseReason,
  ToolMessage,
  validateToolCall,
  DiffChunk,
} from "../../../services/refact";
import { capsApi } from "../../../services/refact";

const createChatThread = (
  tool_use: ToolUse,
  integration?: IntegrationMeta | null,
  mode?: ChatModeId,
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
    increase_max_tokens: false,
    include_project_info: true,
    context_tokens_cap: undefined,
    auto_enrichment_enabled: true,
  };
};

const createThreadRuntime = (
  tool_use: ToolUse,
  integration?: IntegrationMeta | null,
  mode?: ChatModeId,
): ChatThreadRuntime => {
  return {
    thread: createChatThread(tool_use, integration, mode),
    streaming: false,
    waiting_for_response: false,
    prevent_send: false,
    error: null,
    queued_items: [],
    send_immediately: false,
    attached_images: [],
    attached_text_files: [],
    background_agents: {},
    confirmation: {
      pause: false,
      pause_reasons: [],
      status: {
        wasInteracted: false,
        confirmationStatus: true,
      },
    },
    snapshot_received: false,
    task_widget_expanded: false,
    memory_enrichment_user_touched: false,
    manual_preview_items: [],
    manual_preview_ran: false,
  };
};

function createPersistedThreadRuntime(
  tab: ReturnType<typeof loadPersistedChatTabs>["tabs"][number],
  fallbackToolUse: ToolUse,
): ChatThreadRuntime {
  const toolUse = tab.tool_use ?? fallbackToolUse;
  const runtime = createThreadRuntime(
    toolUse,
    null,
    normalizeLegacyMode(tab.mode),
  );

  runtime.thread.id = tab.id;
  runtime.thread.title = tab.title ?? "New Chat";
  runtime.thread.tool_use = toolUse;
  runtime.thread.mode = normalizeLegacyMode(tab.mode);
  runtime.session_state = tab.session_state;

  if (tab.is_buddy_chat) {
    runtime.thread.buddy_meta = {
      is_buddy_chat: true,
      buddy_chat_kind: "conversation",
      workflow_id: null,
    };
  }

  return runtime;
}

const getThreadMode = ({
  integration,
}: {
  integration?: IntegrationMeta | null;
}) => {
  if (integration) return "configurator";
  return "agent";
};

const normalizeMessage = (msg: ChatMessages[number]): ChatMessages[number] => {
  if (msg.role === "diff" && typeof msg.content === "string") {
    try {
      const parsed: unknown = JSON.parse(msg.content);
      if (Array.isArray(parsed)) {
        return {
          ...msg,
          content: parsed as DiffChunk[],
        } as ChatMessages[number];
      }
    } catch {
      // ignore
    }
  }
  return msg;
};

const createInitialState = (): Chat => {
  const persistedTabs = loadPersistedChatTabs();
  const threads: Chat["threads"] = {};

  for (const tab of persistedTabs.tabs) {
    if (tab.is_buddy_chat) continue;
    threads[tab.id] = createPersistedThreadRuntime(tab, "agent");
  }

  const openThreadIds = persistedTabs.openThreadIds.filter(
    (id) => threads[id] !== undefined,
  );
  const currentThreadId = openThreadIds.includes(persistedTabs.currentThreadId)
    ? persistedTabs.currentThreadId
    : openThreadIds[openThreadIds.length - 1] ?? "";

  return {
    current_thread_id: currentThreadId,
    open_thread_ids: openThreadIds,
    threads,
    system_prompt: {},
    tool_use: "agent",
    checkpoints_enabled: true,
    follow_ups_enabled: undefined,
    sse_refresh_requested: null,
    stream_version: 0,
  };
};

const initialState = createInitialState();

const getRuntime = (
  state: Draft<Chat>,
  chatId: string,
): Draft<ChatThreadRuntime> | null => {
  return state.threads[chatId] ?? null;
};

const getCurrentRuntime = (
  state: Draft<Chat>,
): Draft<ChatThreadRuntime> | null => {
  return getRuntime(state, state.current_thread_id);
};

function rebuildMessageIndexById(
  messages: ChatMessages,
): Record<string, number> {
  const index: Record<string, number> = Object.create(null) as Record<
    string,
    number
  >;
  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    if ("message_id" in msg && msg.message_id) {
      index[msg.message_id] = i;
    }
  }
  return index;
}

function findMessageIndexById(
  rt: Draft<ChatThreadRuntime>,
  messageId: string,
): number {
  const indexed = rt.message_index_by_id?.[messageId];
  if (indexed != null) {
    const maybeMsg = rt.thread.messages[indexed];
    if ("message_id" in maybeMsg && maybeMsg.message_id === messageId) {
      return indexed;
    }
  }
  return rt.thread.messages.findIndex(
    (m) => "message_id" in m && m.message_id === messageId,
  );
}

function parseEventSeq(seq: string): bigint | null {
  if (!/^\d+$/.test(seq)) return null;
  try {
    return BigInt(seq);
  } catch {
    return null;
  }
}

function isWorktreeMeta(value: unknown): value is WorktreeMeta {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.id === "string" &&
    typeof record.kind === "string" &&
    typeof record.root === "string" &&
    typeof record.source_workspace_root === "string" &&
    typeof record.repo_root === "string" &&
    typeof record.enforce === "boolean"
  );
}

function isBackgroundAgentTerminal(status: string): boolean {
  return (
    status === "completed" ||
    status === "failed" ||
    status === "cancelled" ||
    status === "interrupted"
  );
}

function shouldReplaceBackgroundAgent(
  existing: { status: string; change_seq: number } | undefined,
  incoming: { status: string; change_seq: number },
): boolean {
  if (!existing) return true;
  if (incoming.change_seq > existing.change_seq) return true;
  if (incoming.change_seq < existing.change_seq) return false;
  return (
    isBackgroundAgentTerminal(incoming.status) &&
    !isBackgroundAgentTerminal(existing.status)
  );
}

function valueOrExisting<T>(value: T | undefined, existing: T): T {
  if (value === undefined) return existing;
  return value;
}

function applyRestoredThread(
  rt: Draft<ChatThreadRuntime>,
  payload: ReturnType<typeof restoreChat>["payload"],
  fallbackToolUse: ToolUse,
) {
  const messages = payload.messages.map(normalizeMessage);
  const existing = rt.thread;
  rt.thread = {
    ...existing,
    id: payload.id,
    messages,
    model: valueOrExisting(payload.model, existing.model),
    title: valueOrExisting(payload.title, existing.title),
    tool_use: payload.tool_use ?? existing.tool_use ?? fallbackToolUse,
    mode: normalizeLegacyMode(payload.mode ?? existing.mode),
    buddy_meta: valueOrExisting(payload.buddy_meta, existing.buddy_meta),
    boost_reasoning: valueOrExisting(
      payload.boost_reasoning,
      existing.boost_reasoning,
    ),
    context_tokens_cap: valueOrExisting(
      payload.context_tokens_cap,
      existing.context_tokens_cap,
    ),
    include_project_info: valueOrExisting(
      payload.include_project_info,
      existing.include_project_info,
    ),
    increase_max_tokens: valueOrExisting(
      payload.increase_max_tokens,
      existing.increase_max_tokens,
    ),
    project_name: valueOrExisting(payload.project_name, existing.project_name),
    isTitleGenerated: valueOrExisting(
      payload.isTitleGenerated,
      existing.isTitleGenerated,
    ),
    new_chat_suggested: valueOrExisting(
      payload.new_chat_suggested,
      existing.new_chat_suggested,
    ),
    createdAt: valueOrExisting(payload.createdAt, existing.createdAt),
    updatedAt: valueOrExisting(payload.updatedAt, existing.updatedAt),
    last_user_message_id: valueOrExisting(
      payload.last_user_message_id,
      existing.last_user_message_id,
    ),
    parent_id: payload.parent_id ?? existing.parent_id,
    link_type: payload.link_type ?? existing.link_type,
    root_chat_id: payload.root_chat_id ?? existing.root_chat_id,
    worktree: valueOrExisting(payload.worktree, existing.worktree),
    task_meta: valueOrExisting(payload.task_meta, existing.task_meta),
    is_task_chat: valueOrExisting(payload.is_task_chat, existing.is_task_chat),
    auto_approve_editing_tools: valueOrExisting(
      payload.auto_approve_editing_tools,
      existing.auto_approve_editing_tools,
    ),
    auto_approve_dangerous_commands: valueOrExisting(
      payload.auto_approve_dangerous_commands,
      existing.auto_approve_dangerous_commands,
    ),
    reasoning_effort: valueOrExisting(
      payload.reasoning_effort,
      existing.reasoning_effort,
    ),
    thinking_budget: valueOrExisting(
      payload.thinking_budget,
      existing.thinking_budget,
    ),
    temperature: valueOrExisting(payload.temperature, existing.temperature),
    frequency_penalty: valueOrExisting(
      payload.frequency_penalty,
      existing.frequency_penalty,
    ),
    max_tokens: valueOrExisting(payload.max_tokens, existing.max_tokens),
    parallel_tool_calls: valueOrExisting(
      payload.parallel_tool_calls,
      existing.parallel_tool_calls,
    ),
    auto_enrichment_enabled: valueOrExisting(
      payload.auto_enrichment_enabled,
      existing.auto_enrichment_enabled,
    ),
    auto_compact_enabled: valueOrExisting(
      payload.auto_compact_enabled,
      existing.auto_compact_enabled,
    ),
  };
  rt.streaming = false;
  rt.waiting_for_response = false;
  rt.prevent_send = false;
  rt.error = null;
  rt.session_state = payload.session_state ?? "idle";
  rt.confirmation.pause = false;
  rt.confirmation.pause_reasons = [];
  rt.confirmation.status.wasInteracted = false;
  rt.confirmation.status.confirmationStatus = true;
  rt.message_index_by_id = rebuildMessageIndexById(messages);
}

export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(hydratePersistedChatTabs, (state) => {
    const persistedTabs = loadPersistedChatTabs();
    const threads: Chat["threads"] = {};

    for (const tab of persistedTabs.tabs) {
      if (tab.is_buddy_chat) continue;
      threads[tab.id] =
        state.threads[tab.id] ?? createPersistedThreadRuntime(tab, "agent");
    }

    const openThreadIds = persistedTabs.openThreadIds.filter(
      (id) => threads[id] !== undefined,
    );
    const currentThreadId = openThreadIds.includes(
      persistedTabs.currentThreadId,
    )
      ? persistedTabs.currentThreadId
      : openThreadIds[openThreadIds.length - 1] ?? "";

    state.threads = threads;
    state.open_thread_ids = openThreadIds;
    state.current_thread_id = currentThreadId;
  });

  builder.addCase(setToolUse, (state, action) => {
    state.tool_use = action.payload;
  });

  builder.addCase(setThreadMode, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt && rt.thread.messages.length === 0) {
      rt.thread.mode = action.payload.mode;
      const defaults = action.payload.threadDefaults;
      if (defaults) {
        if (defaults.include_project_info !== undefined) {
          rt.thread.include_project_info = defaults.include_project_info;
        }
        if (defaults.checkpoints_enabled !== undefined) {
          rt.thread.checkpoints_enabled = defaults.checkpoints_enabled;
        }
        if (defaults.auto_approve_editing_tools !== undefined) {
          rt.thread.auto_approve_editing_tools =
            defaults.auto_approve_editing_tools;
        }
        if (defaults.auto_approve_dangerous_commands !== undefined) {
          rt.thread.auto_approve_dangerous_commands =
            defaults.auto_approve_dangerous_commands;
        }
      }
    }
  });

  builder.addCase(setThreadWorktree, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.thread.worktree = action.payload.worktree;
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
    const mode = getThreadMode({});
    const lastParams = getLastThreadParams(mode);
    const newRuntime = createThreadRuntime(state.tool_use, null, mode);

    newRuntime.thread.model = lastParams.model ?? currentRt?.thread.model ?? "";
    newRuntime.thread.boost_reasoning =
      lastParams.boost_reasoning ?? currentRt?.thread.boost_reasoning ?? false;
    newRuntime.thread.reasoning_effort = lastParams.reasoning_effort;
    newRuntime.thread.thinking_budget = lastParams.thinking_budget;
    newRuntime.thread.max_tokens = lastParams.max_tokens;
    newRuntime.thread.increase_max_tokens =
      lastParams.increase_max_tokens ??
      currentRt?.thread.increase_max_tokens ??
      false;
    newRuntime.thread.include_project_info =
      lastParams.include_project_info ??
      currentRt?.thread.include_project_info ??
      true;

    if (action.payload?.title) {
      newRuntime.thread.title = action.payload.title;
    }

    const newId = newRuntime.thread.id;
    state.threads[newId] = newRuntime;
    state.open_thread_ids.push(newId);
    state.current_thread_id = newId;
  });

  builder.addCase(createChatWithId, (state, action) => {
    const {
      id,
      title,
      isTaskChat,
      mode,
      taskMeta,
      model,
      parentId,
      linkType,
      worktree,
    } = action.payload;
    const existingRt = state.threads[id];

    if (existingRt) {
      if (isTaskChat) {
        existingRt.thread.is_task_chat = true;
        state.open_thread_ids = state.open_thread_ids.filter(
          (tid) => tid !== id,
        );
      }
      if (title && !existingRt.thread.title) {
        existingRt.thread.title = title;
      }
      if (mode) {
        existingRt.thread.mode = normalizeLegacyMode(mode);
      }
      if (taskMeta) {
        existingRt.thread.task_meta = taskMeta;
      }
      if (model && !existingRt.thread.model) {
        existingRt.thread.model = model;
      }
      if (worktree !== undefined) {
        existingRt.thread.worktree = worktree;
      }
      if (parentId !== undefined) {
        existingRt.thread.parent_id = parentId;
      }
      if (linkType !== undefined) {
        existingRt.thread.link_type = linkType;
      }
      state.current_thread_id = id;
      return;
    }

    const currentRt = getCurrentRuntime(state);
    const effectiveMode = normalizeLegacyMode(mode ?? getThreadMode({}));
    const lastParams = getLastThreadParams(effectiveMode);
    const newRuntime = createThreadRuntime(state.tool_use, null, effectiveMode);

    newRuntime.thread.id = id;
    newRuntime.thread.model =
      model ?? lastParams.model ?? currentRt?.thread.model ?? "";
    newRuntime.thread.boost_reasoning =
      lastParams.boost_reasoning ?? currentRt?.thread.boost_reasoning ?? false;
    newRuntime.thread.reasoning_effort = lastParams.reasoning_effort;
    newRuntime.thread.thinking_budget = lastParams.thinking_budget;
    newRuntime.thread.max_tokens = lastParams.max_tokens;
    newRuntime.thread.increase_max_tokens =
      lastParams.increase_max_tokens ??
      currentRt?.thread.increase_max_tokens ??
      false;
    newRuntime.thread.include_project_info =
      lastParams.include_project_info ??
      currentRt?.thread.include_project_info ??
      true;

    if (title) {
      newRuntime.thread.title = title;
    }
    if (isTaskChat) {
      newRuntime.thread.is_task_chat = true;
    }
    if (taskMeta) {
      newRuntime.thread.task_meta = taskMeta;
    }
    if (worktree !== undefined) {
      newRuntime.thread.worktree = worktree;
    }
    if (parentId !== undefined) {
      newRuntime.thread.parent_id = parentId;
    }
    if (linkType !== undefined) {
      newRuntime.thread.link_type = linkType;
    }

    state.threads[id] = newRuntime;
    if (!isTaskChat) {
      state.open_thread_ids.push(id);
    }
    state.current_thread_id = id;
  });

  builder.addCase(backUpMessages, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.error = null;
      rt.thread.messages = action.payload.messages;
    }
  });

  builder.addCase(setAutoApproveEditingTools, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.auto_approve_editing_tools = action.payload.value;
  });

  builder.addCase(setAutoApproveDangerousCommands, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.auto_approve_dangerous_commands = action.payload.value;
  });

  builder.addCase(setIsNewChatSuggested, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt)
      rt.thread.new_chat_suggested = { wasSuggested: action.payload.value };
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
    if (rt) {
      rt.thread.boost_reasoning = action.payload.value;
      // Reasoning implies temperature must be unset (treated as "None").
      if (action.payload.value) {
        rt.thread.temperature = undefined;
      }
    }
  });

  builder.addCase(setReasoningEffort, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.thread.reasoning_effort = action.payload.value;
      if (action.payload.value != null) {
        rt.thread.temperature = null;
      }
    }
  });

  builder.addCase(setThinkingBudget, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.thread.thinking_budget = action.payload.value;
      if (action.payload.value != null) {
        rt.thread.temperature = null;
      }
    }
  });

  builder.addCase(setTemperature, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.temperature = action.payload.value;
  });

  builder.addCase(setFrequencyPenalty, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.frequency_penalty = action.payload.value;
  });

  builder.addCase(setMaxTokens, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.max_tokens = action.payload.value;
  });

  builder.addCase(setParallelToolCalls, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.parallel_tool_calls = action.payload.value;
  });

  builder.addCase(setTaskWidgetExpanded, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) rt.task_widget_expanded = action.payload.expanded;
  });

  builder.addCase(openBuddyChat, (state, action) => {
    const { chat_id, title } = action.payload;
    const existingRt = getRuntime(state, chat_id);
    if (existingRt) {
      state.current_thread_id = chat_id;
      return;
    }
    const newRuntime = createThreadRuntime(state.tool_use, null, "buddy");
    newRuntime.thread.id = chat_id;
    newRuntime.thread.buddy_meta = {
      is_buddy_chat: true,
      buddy_chat_kind: "conversation",
      workflow_id: null,
    };
    if (title) newRuntime.thread.title = title;
    state.threads[chat_id] = newRuntime;
    state.current_thread_id = chat_id;
  });

  builder.addCase(newBuddyChatAction, (state, action) => {
    const { chat_id } = action.payload;
    const existingRt = getRuntime(state, chat_id);
    if (existingRt) {
      state.current_thread_id = chat_id;
      return;
    }
    const newRuntime = createThreadRuntime(state.tool_use, null, "buddy");
    newRuntime.thread.id = chat_id;
    newRuntime.thread.buddy_meta = {
      is_buddy_chat: true,
      buddy_chat_kind: "conversation",
      workflow_id: null,
    };
    state.threads[chat_id] = newRuntime;
    state.current_thread_id = chat_id;
  });

  builder.addCase(setAutoEnrichmentEnabled, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.auto_enrichment_enabled = action.payload.value;
  });

  builder.addCase(setAutoCompactEnabled, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.auto_compact_enabled = action.payload.value;
  });

  builder.addCase(markMemoryEnrichmentUserTouched, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.memory_enrichment_user_touched = true;
  });

  builder.addCase(setManualPreviewItems, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.manual_preview_items = action.payload.items;
      rt.manual_preview_ran = true;
    }
  });

  builder.addCase(removeManualPreviewItem, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.manual_preview_items = rt.manual_preview_items.filter(
        (_, i) => i !== action.payload.index,
      );
    }
  });

  builder.addCase(clearManualPreviewItems, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) {
      rt.manual_preview_items = [];
      rt.manual_preview_ran = false;
    }
  });

  builder.addCase(setLastUserMessageId, (state, action) => {
    const rt = getRuntime(state, action.payload.chatId);
    if (rt) rt.thread.last_user_message_id = action.payload.messageId;
  });

  builder.addCase(removeChatFromCache, (state, action) => {
    const id = action.payload.id;
    const rt = state.threads[id];
    if (rt && !rt.streaming && !rt.confirmation.pause) {
      const { [id]: _, ...rest } = state.threads;
      state.threads = rest;
      state.open_thread_ids = state.open_thread_ids.filter((tid) => tid !== id);
    }
  });

  builder.addCase(closeThread, (state, action) => {
    const id = action.payload.id;
    const force = action.payload.force ?? false;
    state.open_thread_ids = state.open_thread_ids.filter((tid) => tid !== id);
    const rt = state.threads[id];
    if (
      rt &&
      (force ||
        (!rt.streaming && !rt.waiting_for_response && !rt.confirmation.pause))
    ) {
      const { [id]: _, ...rest } = state.threads;
      state.threads = rest;
    }
    if (state.current_thread_id === id) {
      state.current_thread_id = state.open_thread_ids[0] ?? "";
    }
  });

  builder.addCase(restoreChat, (state, action) => {
    const isBuddyChat = Boolean(action.payload.buddy_meta?.is_buddy_chat);
    const existingRt = getRuntime(state, action.payload.id);
    if (existingRt) {
      applyRestoredThread(existingRt, action.payload, state.tool_use);
      if (isBuddyChat || existingRt.thread.is_task_chat) {
        state.open_thread_ids = state.open_thread_ids.filter(
          (id) => id !== action.payload.id,
        );
      } else if (!state.open_thread_ids.includes(action.payload.id)) {
        state.open_thread_ids.push(action.payload.id);
      }
      state.current_thread_id = action.payload.id;
      return;
    }

    const newRuntime: ChatThreadRuntime = {
      thread: {
        id: action.payload.id,
        messages: [],
        model: "",
        title: "",
        tool_use: state.tool_use,
        mode: normalizeLegacyMode(action.payload.mode),
        new_chat_suggested: { wasSuggested: false },
      },
      streaming: false,
      waiting_for_response: false,
      prevent_send: false,
      error: null,
      queued_items: [],
      send_immediately: false,
      attached_images: [],
      attached_text_files: [],
      background_agents: {},
      confirmation: {
        pause: false,
        pause_reasons: [],
        status: {
          wasInteracted: false,
          confirmationStatus: true,
        },
      },
      snapshot_received: false,
      task_widget_expanded: false,
      memory_enrichment_user_touched: false,
      manual_preview_items: [],
      manual_preview_ran: false,
    };
    applyRestoredThread(newRuntime, action.payload, state.tool_use);

    state.threads[action.payload.id] = newRuntime;
    if (!isBuddyChat && !state.open_thread_ids.includes(action.payload.id)) {
      state.open_thread_ids.push(action.payload.id);
    }
    state.current_thread_id = action.payload.id;
  });

  builder.addCase(switchToThread, (state, action) => {
    const { id, openTab } = action.payload;
    const existingRt = getRuntime(state, id);

    if (!existingRt) {
      // eslint-disable-next-line no-console
      console.warn(`[switchToThread] No runtime for ${id}`);
    }

    if (existingRt) {
      const shouldOpenTab =
        openTab !== false && !existingRt.thread.is_task_chat;
      if (shouldOpenTab && !state.open_thread_ids.includes(id)) {
        state.open_thread_ids.push(id);
      }
      state.current_thread_id = id;
    }
  });

  builder.addCase(updateOpenThread, (state, action) => {
    const existingRt = getRuntime(state, action.payload.id);
    if (!existingRt) return;

    const incomingTitle = action.payload.thread.title;
    const incomingGenerated = action.payload.thread.isTitleGenerated;

    if (incomingTitle) {
      if (incomingGenerated === true) {
        if (!existingRt.thread.isTitleGenerated) {
          existingRt.thread.title = incomingTitle;
          existingRt.thread.isTitleGenerated = true;
        }
      } else if (incomingGenerated === false) {
        existingRt.thread.title = incomingTitle;
        existingRt.thread.isTitleGenerated = false;
      }
    }

    const isCurrentThread = action.payload.id === state.current_thread_id;
    if (
      !existingRt.streaming &&
      !existingRt.waiting_for_response &&
      !existingRt.error &&
      !isCurrentThread
    ) {
      const {
        title: _title,
        isTitleGenerated: _isTitleGenerated,
        messages: _messages,
        ...otherFields
      } = action.payload.thread;
      existingRt.thread = {
        ...existingRt.thread,
        ...otherFields,
      };
    }
  });

  builder.addCase(updateChatRuntimeFromSessionState, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (!rt) return;

    const sessionState = action.payload.session_state;

    // When a thread has an active chat SSE subscription (snapshot_received),
    // the chat SSE channel (applyChatEvent → runtime_updated) is the
    // authoritative source for runtime state. The sidebar SSE can deliver
    // stale trajectory events (e.g. "generating") that arrive AFTER the chat
    // SSE has already moved to "waiting_user_input" or "completed", causing
    // boolean flags to be incorrectly overwritten. Skip boolean/flag updates
    // for threads with an active chat SSE; only update session_state for
    // display purposes (tabs, StatusDot).
    if (rt.snapshot_received) {
      // Keep the last known session_state for display (tabs/StatusDot), but do
      // not overwrite streaming/waiting flags.
      rt.session_state = sessionState;
      if (sessionState === "error") {
        rt.error = action.payload.error ?? "An error occurred";
      }
      return;
    }

    rt.session_state = sessionState;
    rt.streaming = sessionState === "generating";
    rt.waiting_for_response =
      sessionState === "generating" ||
      sessionState === "executing_tools" ||
      sessionState === "waiting_ide";
    rt.prevent_send = false;

    if (sessionState === "paused") {
      rt.confirmation.pause = true;
      if (rt.confirmation.pause_reasons.length === 0) {
        state.sse_refresh_requested = action.payload.id;
      }
    } else if (
      sessionState === "idle" ||
      sessionState === "error" ||
      sessionState === "completed" ||
      sessionState === "waiting_user_input"
    ) {
      rt.confirmation.pause = false;
      rt.confirmation.pause_reasons = [];
    }

    if (sessionState === "error") {
      rt.error = action.payload.error ?? "An error occurred";
    } else if (
      sessionState === "idle" ||
      sessionState === "completed" ||
      sessionState === "waiting_user_input"
    ) {
      rt.error = null;
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
    const newRuntime = createThreadRuntime(
      "agent",
      action.payload.integration,
      "configurator",
    );
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

  builder.addCase(markThreadSseError, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (!rt) return;
    rt.is_compressing = false;
    const wasBusy =
      rt.streaming ||
      rt.waiting_for_response ||
      rt.session_state === "generating" ||
      rt.session_state === "executing_tools" ||
      rt.session_state === "waiting_ide";
    if (!wasBusy) return;

    rt.streaming = false;
    rt.waiting_for_response = false;
    rt.prevent_send = false;
    rt.session_state = "error";
    rt.error = action.payload.error;
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
      rt.confirmation.status.confirmationStatus =
        action.payload.confirmationStatus;
    }
  });

  builder.addCase(addThreadImage, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt && rt.attached_images.length < 5) {
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

  builder.addCase(addThreadTextFile, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.attached_text_files.push(action.payload.file);
    }
  });

  builder.addCase(removeThreadTextFileByIndex, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.attached_text_files = rt.attached_text_files.filter(
        (_, index) => index !== action.payload.index,
      );
    }
  });

  builder.addCase(resetThreadTextFiles, (state, action) => {
    const rt = getRuntime(state, action.payload.id);
    if (rt) {
      rt.attached_text_files = [];
    }
  });

  builder.addCase(applyChatEvent, (state, action) => {
    const { chat_id, ...event } = action.payload;

    const rt = getRuntime(state, chat_id);

    switch (event.type) {
      case "snapshot": {
        const existingRuntime = rt;
        const existing = existingRuntime?.thread;
        const snapshotMessages = (event.messages as ChatMessages).map(
          normalizeMessage,
        );
        const backgroundAgents = {
          ...(existingRuntime?.background_agents ?? {}),
        };
        for (const agent of event.background_agents) {
          if (
            shouldReplaceBackgroundAgent(
              backgroundAgents[agent.agent_id],
              agent,
            )
          ) {
            backgroundAgents[agent.agent_id] = agent;
          }
        }

        const backendModel = event.thread.model.trim();
        const backendToolUse = event.thread.tool_use;
        const backendMode = event.thread.mode;

        const snapshotTaskMeta = event.thread.task_meta ?? existing?.task_meta;
        const isTaskChat =
          Boolean(existing?.is_task_chat) || Boolean(snapshotTaskMeta?.task_id);
        const snapshotBuddyMeta =
          (event.thread as { buddy_meta?: ChatThread["buddy_meta"] })
            .buddy_meta ?? existing?.buddy_meta;

        const snapshotTitle = event.thread.title;
        const existingTitle = existingRuntime?.thread.title;
        const snapshotTitleGenerated = event.thread.is_title_generated;
        const existingTitleGenerated =
          existingRuntime?.thread.isTitleGenerated === true;
        const useSnapshotTitle =
          !existingTitle ||
          existingTitle === "New Chat" ||
          (snapshotTitleGenerated && !existingTitleGenerated);

        const thread: ChatThread = {
          id: event.thread.id,
          messages: snapshotMessages,
          model: backendModel || (existing?.model ?? ""),
          title: useSnapshotTitle ? snapshotTitle : existingTitle,
          tool_use: isToolUse(backendToolUse)
            ? backendToolUse
            : existing?.tool_use && isToolUse(existing.tool_use)
              ? existing.tool_use
              : "agent",
          mode: normalizeLegacyMode(backendMode || existing?.mode),
          boost_reasoning: event.thread.boost_reasoning,
          context_tokens_cap:
            event.thread.context_tokens_cap ?? existing?.context_tokens_cap,
          include_project_info: event.thread.include_project_info,
          checkpoints_enabled: event.thread.checkpoints_enabled,
          isTitleGenerated:
            existingRuntime?.thread.isTitleGenerated ??
            event.thread.is_title_generated,
          auto_approve_editing_tools:
            event.thread.auto_approve_editing_tools ??
            existing?.auto_approve_editing_tools ??
            false,
          auto_approve_dangerous_commands:
            event.thread.auto_approve_dangerous_commands ??
            existing?.auto_approve_dangerous_commands ??
            false,
          increase_max_tokens: existing?.increase_max_tokens ?? false,
          new_chat_suggested: { wasSuggested: false },
          is_task_chat: isTaskChat,
          task_meta: snapshotTaskMeta,
          reasoning_effort: isReasoningEffort(event.thread.reasoning_effort)
            ? event.thread.reasoning_effort
            : undefined,
          thinking_budget:
            typeof event.thread.thinking_budget === "number"
              ? event.thread.thinking_budget
              : undefined,
          temperature:
            typeof event.thread.temperature === "number"
              ? event.thread.temperature
              : undefined,
          frequency_penalty:
            typeof event.thread.frequency_penalty === "number"
              ? event.thread.frequency_penalty
              : undefined,
          max_tokens:
            typeof event.thread.max_tokens === "number"
              ? event.thread.max_tokens
              : undefined,
          parallel_tool_calls:
            typeof event.thread.parallel_tool_calls === "boolean"
              ? event.thread.parallel_tool_calls
              : undefined,
          auto_enrichment_enabled:
            typeof event.thread.auto_enrichment_enabled === "boolean"
              ? (event.thread.auto_enrichment_enabled as boolean)
              : existing?.auto_enrichment_enabled ?? false,
          auto_compact_enabled:
            typeof event.thread.auto_compact_enabled === "boolean"
              ? event.thread.auto_compact_enabled
              : existing?.auto_compact_enabled,
          worktree:
            "worktree" in event.thread
              ? event.thread.worktree === null
                ? null
                : isWorktreeMeta(event.thread.worktree)
                  ? event.thread.worktree
                  : existing?.worktree
              : existing?.worktree,
          parent_id: event.thread.parent_id ?? existing?.parent_id,
          link_type: event.thread.link_type ?? existing?.link_type,
          root_chat_id: event.thread.root_chat_id ?? existing?.root_chat_id,
          buddy_meta: snapshotBuddyMeta,
        };

        const snapshotState = event.runtime.state as string;
        const snapshotStreaming = snapshotState === "generating";
        const snapshotWaiting =
          snapshotState === "generating" ||
          snapshotState === "executing_tools" ||
          snapshotState === "waiting_ide";

        const newRt: ChatThreadRuntime = {
          thread,
          session_state: snapshotState,
          streaming: snapshotStreaming,
          waiting_for_response: snapshotWaiting,
          prevent_send: false,
          error: event.runtime.error ?? null,
          queued_items: event.runtime
            .queued_items as ChatThreadRuntime["queued_items"],
          send_immediately: existingRuntime?.send_immediately ?? false,
          attached_images: existingRuntime?.attached_images ?? [],
          attached_text_files: existingRuntime?.attached_text_files ?? [],
          background_agents: backgroundAgents,
          confirmation: {
            pause: event.runtime.paused,
            pause_reasons: event.runtime
              .pause_reasons as ToolConfirmationPauseReason[],
            status: existingRuntime?.confirmation.status ?? {
              wasInteracted: false,
              confirmationStatus: true,
            },
          },
          snapshot_received: true,
          is_compressing:
            typeof event.runtime.is_compressing === "boolean"
              ? event.runtime.is_compressing
              : false,
          task_widget_expanded: existingRuntime?.task_widget_expanded ?? false,
          last_applied_seq: event.seq,
          message_index_by_id: rebuildMessageIndexById(snapshotMessages),
          memory_enrichment_user_touched:
            existingRuntime?.memory_enrichment_user_touched ?? false,
          manual_preview_items: existingRuntime?.manual_preview_items ?? [],
          manual_preview_ran: existingRuntime?.manual_preview_ran ?? false,
        };

        state.threads[chat_id] = newRt;

        const isBuddyChat = Boolean(snapshotBuddyMeta?.is_buddy_chat);
        if (
          !isTaskChat &&
          !isBuddyChat &&
          !state.open_thread_ids.includes(chat_id)
        ) {
          state.open_thread_ids.push(chat_id);
        }
        if (!state.current_thread_id) {
          state.current_thread_id = chat_id;
        }
        break;
      }

      case "background_agent_updated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        if (
          shouldReplaceBackgroundAgent(
            rt.background_agents[event.agent.agent_id],
            event.agent,
          )
        ) {
          rt.background_agents[event.agent.agent_id] = event.agent;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "thread_updated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const { type: _, ...params } = event;
        if ("model" in params && typeof params.model === "string")
          rt.thread.model = params.model;
        if ("mode" in params && typeof params.mode === "string") {
          rt.thread.mode = normalizeLegacyMode(params.mode);
        }
        if (
          "boost_reasoning" in params &&
          typeof params.boost_reasoning === "boolean"
        )
          rt.thread.boost_reasoning = params.boost_reasoning;
        if ("tool_use" in params && typeof params.tool_use === "string") {
          rt.thread.tool_use = isToolUse(params.tool_use)
            ? params.tool_use
            : rt.thread.tool_use;
        }
        if ("context_tokens_cap" in params) {
          rt.thread.context_tokens_cap =
            params.context_tokens_cap == null
              ? undefined
              : (params.context_tokens_cap as number);
        }
        if (
          "include_project_info" in params &&
          typeof params.include_project_info === "boolean"
        )
          rt.thread.include_project_info = params.include_project_info;
        if (
          "checkpoints_enabled" in params &&
          typeof params.checkpoints_enabled === "boolean"
        )
          rt.thread.checkpoints_enabled = params.checkpoints_enabled;
        if (
          "auto_approve_editing_tools" in params &&
          typeof params.auto_approve_editing_tools === "boolean"
        )
          rt.thread.auto_approve_editing_tools =
            params.auto_approve_editing_tools;
        if (
          "auto_approve_dangerous_commands" in params &&
          typeof params.auto_approve_dangerous_commands === "boolean"
        )
          rt.thread.auto_approve_dangerous_commands =
            params.auto_approve_dangerous_commands;
        if ("reasoning_effort" in params) {
          rt.thread.reasoning_effort =
            params.reasoning_effort == null
              ? undefined
              : (params.reasoning_effort as ChatThread["reasoning_effort"]);
        }
        if ("thinking_budget" in params) {
          rt.thread.thinking_budget =
            params.thinking_budget == null
              ? undefined
              : (params.thinking_budget as number);
        }
        if ("temperature" in params) {
          rt.thread.temperature =
            params.temperature == null
              ? undefined
              : (params.temperature as number);
        }
        if ("frequency_penalty" in params) {
          rt.thread.frequency_penalty =
            params.frequency_penalty == null
              ? undefined
              : (params.frequency_penalty as number);
        }
        if ("max_tokens" in params) {
          rt.thread.max_tokens =
            params.max_tokens == null
              ? undefined
              : (params.max_tokens as number);
        }
        if ("parallel_tool_calls" in params) {
          rt.thread.parallel_tool_calls =
            params.parallel_tool_calls == null
              ? undefined
              : (params.parallel_tool_calls as boolean);
        }
        if ("auto_enrichment_enabled" in params) {
          const rawAe = params.auto_enrichment_enabled;
          rt.thread.auto_enrichment_enabled =
            rawAe == null ? undefined : (rawAe as boolean);
        }
        if ("auto_compact_enabled" in params) {
          const rawCompact = params.auto_compact_enabled;
          rt.thread.auto_compact_enabled =
            rawCompact == null ? undefined : (rawCompact as boolean);
        }
        if ("worktree" in params) {
          const rawWorktree = params.worktree;
          if (rawWorktree == null) {
            rt.thread.worktree = null;
          } else if (isWorktreeMeta(rawWorktree)) {
            rt.thread.worktree = rawWorktree;
          }
        }
        if ("task_meta" in params && params.task_meta != null) {
          rt.thread.task_meta = params.task_meta as ChatThread["task_meta"];
          rt.thread.is_task_chat = true;
          state.open_thread_ids = state.open_thread_ids.filter(
            (id) => id !== chat_id,
          );
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "message_added": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const msg = normalizeMessage(event.message);
        const messageId = "message_id" in msg ? msg.message_id : null;
        if (messageId) {
          const existingIdx = findMessageIndexById(rt, messageId);
          if (existingIdx >= 0) {
            const existing = rt.thread.messages[existingIdx];
            if (isAssistantMessage(existing) && isAssistantMessage(msg)) {
              const merged: AssistantMessage = {
                ...msg,
                tool_calls: msg.tool_calls ?? existing.tool_calls,
                server_executed_tools:
                  msg.server_executed_tools ?? existing.server_executed_tools,
                server_content_blocks:
                  msg.server_content_blocks ?? existing.server_content_blocks,
                reasoning_content:
                  msg.reasoning_content ?? existing.reasoning_content,
                thinking_blocks:
                  msg.thinking_blocks ?? existing.thinking_blocks,
                citations: msg.citations ?? existing.citations,
                usage: msg.usage ?? existing.usage,
                extra: msg.extra ?? existing.extra,
                finish_reason: msg.finish_reason ?? existing.finish_reason,
              };
              rt.thread.messages[existingIdx] = merged;
            } else {
              rt.thread.messages[existingIdx] = msg;
            }
            rt.message_index_by_id = rebuildMessageIndexById(
              rt.thread.messages,
            );
            rt.last_applied_seq = event.seq;
            break;
          }
        }
        const clampedIndex = Math.max(
          0,
          Math.min(event.index, rt.thread.messages.length),
        );
        rt.thread.messages.splice(clampedIndex, 0, msg);
        rt.message_index_by_id = rebuildMessageIndexById(rt.thread.messages);
        rt.last_applied_seq = event.seq;
        break;
      }

      case "process_completed": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "message_updated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const idx = findMessageIndexById(rt, event.message_id);
        if (idx >= 0) {
          rt.thread.messages[idx] = normalizeMessage(event.message);
          rt.message_index_by_id = rebuildMessageIndexById(rt.thread.messages);
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "message_removed": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.thread.messages = rt.thread.messages.filter(
          (m) => !("message_id" in m) || m.message_id !== event.message_id,
        );
        rt.message_index_by_id = rebuildMessageIndexById(rt.thread.messages);
        rt.last_applied_seq = event.seq;
        break;
      }

      case "messages_truncated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const clampedIndex = Math.max(
          0,
          Math.min(event.from_index, rt.thread.messages.length),
        );
        rt.thread.messages = rt.thread.messages.slice(0, clampedIndex);
        rt.message_index_by_id = rebuildMessageIndexById(rt.thread.messages);
        rt.last_applied_seq = event.seq;
        break;
      }

      case "stream_started": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const existingIdx = findMessageIndexById(rt, event.message_id);
        rt.streaming = true;
        rt.waiting_for_response = true;
        rt.session_state = "generating";
        if (existingIdx < 0) {
          rt.thread.messages.push({
            role: "assistant",
            content: "",
            message_id: event.message_id,
          } as ChatMessages[number]);
          rt.message_index_by_id = rebuildMessageIndexById(rt.thread.messages);
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "stream_delta": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const msgIdx = findMessageIndexById(rt, event.message_id);
        if (msgIdx >= 0) {
          const msg = rt.thread.messages[msgIdx];
          rt.thread.messages[msgIdx] = applyDeltaOps(
            msg as Parameters<typeof applyDeltaOps>[0],
            event.ops,
          );
          state.stream_version = (state.stream_version + 1) % 1_000_000;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "stream_finished": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.streaming = false;
        if (
          event.finish_reason === "stop" ||
          event.finish_reason === "length" ||
          event.finish_reason === "abort" ||
          event.finish_reason === "error"
        ) {
          rt.waiting_for_response = false;
          rt.session_state = "idle";
        } else {
          // tool_calls or other finish reasons: tools about to execute
          rt.session_state = "executing_tools";
        }
        const msgIdx = findMessageIndexById(rt, event.message_id);
        if (msgIdx >= 0 && isAssistantMessage(rt.thread.messages[msgIdx])) {
          const msg = rt.thread.messages[msgIdx] as AssistantMessage;
          if (event.finish_reason && !msg.finish_reason) {
            msg.finish_reason =
              event.finish_reason as AssistantMessage["finish_reason"];
          }
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "pause_required": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.streaming = false;
        rt.waiting_for_response = false;
        rt.session_state = "paused";
        rt.confirmation.pause = true;
        rt.confirmation.pause_reasons =
          event.reasons as ToolConfirmationPauseReason[];
        rt.confirmation.status.wasInteracted = false;
        rt.confirmation.status.confirmationStatus = false;
        rt.last_applied_seq = event.seq;
        break;
      }

      case "pause_cleared": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.confirmation.pause = false;
        rt.confirmation.pause_reasons = [];
        rt.confirmation.status.wasInteracted = false;
        rt.confirmation.status.confirmationStatus = true;
        rt.last_applied_seq = event.seq;
        break;
      }

      case "ide_tool_required": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "subchat_update": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        for (const msg of rt.thread.messages) {
          if (!isAssistantMessage(msg) || !msg.tool_calls) continue;
          const tc = msg.tool_calls.find((t) => t.id === event.tool_call_id);
          if (tc) {
            if (event.subchat_id === "") {
              tc.subchat = undefined;
              tc.subchat_log = [];
              tc.attached_files = [];
            } else {
              tc.subchat = event.subchat_id;
              const isToolNotification = event.subchat_id.includes("/tool:");
              if (!isToolNotification) {
                // Streaming progress: keep only the latest entry so UI doesn't
                // accumulate stale partial text.
                tc.subchat_log = [event.subchat_id];
              }
            }
            if (event.attached_files && event.attached_files.length > 0) {
              tc.attached_files = [
                ...(tc.attached_files ?? []),
                ...event.attached_files.filter(
                  (f) => !tc.attached_files?.includes(f),
                ),
              ];
            }
            break;
          }
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "ack": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "queue_updated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        rt.queued_items =
          event.queued_items as ChatThreadRuntime["queued_items"];
        rt.last_applied_seq = event.seq;
        break;
      }

      case "runtime_updated": {
        if (!rt) break;
        const eventSeq = parseEventSeq(event.seq);
        const lastSeq =
          rt.last_applied_seq != null
            ? parseEventSeq(rt.last_applied_seq)
            : null;
        if (eventSeq != null && lastSeq != null && eventSeq <= lastSeq) {
          break;
        }
        const newState = event.state;
        rt.session_state = newState;

        // Update streaming/waiting flags based on state
        switch (newState) {
          case "idle":
          case "completed":
          case "waiting_user_input":
          case "error":
            rt.streaming = false;
            rt.waiting_for_response = false;
            break;
          case "generating":
            rt.streaming = true;
            rt.waiting_for_response = true;
            break;
          case "executing_tools":
          case "waiting_ide":
            rt.streaming = false;
            rt.waiting_for_response = true;
            break;
          case "paused":
            rt.streaming = false;
            rt.waiting_for_response = false;
            // Note: pause_reasons are set via pause_required event
            break;
        }

        // Update error state
        if (newState === "error" && event.error) {
          rt.error = event.error;
        } else if (newState !== "error") {
          rt.error = null;
        }
        if (typeof event.is_compressing === "boolean") {
          rt.is_compressing = event.is_compressing;
        } else if (
          newState === "idle" ||
          newState === "completed" ||
          newState === "error" ||
          newState === "waiting_user_input"
        ) {
          rt.is_compressing = false;
        }
        rt.last_applied_seq = event.seq;
        break;
      }

      case "browser_context_oversize": {
        if (!rt) break;
        rt.streaming = false;
        rt.waiting_for_response = false;
        rt.last_applied_seq = event.seq;
        break;
      }
    }
  });

  builder.addCase(requestSseRefresh, (state, action) => {
    state.sse_refresh_requested = action.payload.chatId;
  });

  builder.addCase(clearSseRefreshRequest, (state) => {
    state.sse_refresh_requested = null;
  });

  builder.addMatcher(
    capsApi.endpoints.getCaps.matchFulfilled,
    (state, action) => {
      const defaultModel = action.payload.chat_default_model;
      const rt = getCurrentRuntime(state);
      if (!rt) return;

      const model = rt.thread.model || defaultModel;
      if (!(model in action.payload.chat_models)) return;

      if (!rt.thread.model) {
        rt.thread.model = defaultModel;
      }

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
    (d) => isToolMessage(d) && d.tool_call_id === toolCallId,
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
    typeof maybeToolResult.content === "string"
  ) {
    maybeToolResult.content = message;
    return;
  } else if (
    maybeToolResult &&
    isToolMessage(maybeToolResult) &&
    Array.isArray(maybeToolResult.content)
  ) {
    maybeToolResult.content.push({
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
    tool_call_id: toolCallId,
    content: message,
    tool_failed: false,
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
