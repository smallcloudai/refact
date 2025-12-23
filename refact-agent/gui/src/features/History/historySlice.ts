import {
  createSlice,
  PayloadAction,
  createListenerMiddleware,
} from "@reduxjs/toolkit";
import {
  backUpMessages,
  chatAskedQuestion,
  ChatThread,
  doneStreaming,
  isLspChatMode,
  maybeAppendToolCallResultFromIdeToMessages,
  restoreChat,
  setChatMode,
  SuggestedChat,
} from "../Chat/Thread";
import {
  trajectoriesApi,
  chatThreadToTrajectoryData,
  TrajectoryData,
  trajectoryDataToChatThread,
} from "../../services/refact";
import { AppDispatch, RootState } from "../../app/store";
import { ideToolCallResponse } from "../../hooks/useEventBusForIDE";

export type ChatHistoryItem = Omit<ChatThread, "new_chat_suggested"> & {
  createdAt: string;
  updatedAt: string;
  title: string;
  isTitleGenerated?: boolean;
  new_chat_suggested?: SuggestedChat;
};

export type HistoryMeta = Pick<
  ChatHistoryItem,
  "id" | "title" | "createdAt" | "model" | "updatedAt"
> & { userMessageCount: number };

export type HistoryState = Record<string, ChatHistoryItem>;

const initialState: HistoryState = {};

function getFirstUserContentFromChat(messages: ChatThread["messages"]): string {
  const message = messages.find(
    (msg): msg is ChatThread["messages"][number] & { role: "user" } =>
      msg.role === "user",
  );
  if (!message) return "New Chat";
  if (typeof message.content === "string") {
    return message.content.replace(/^\s+/, "").slice(0, 100);
  }

  const firstUserInput = message.content.find((item) => {
    if ("m_type" in item && item.m_type === "text") {
      return true;
    }
    if ("type" in item && item.type === "text") {
      return true;
    }
    return false;
  });
  if (!firstUserInput) return "New Chat";
  const text =
    "m_content" in firstUserInput
      ? firstUserInput.m_content
      : "text" in firstUserInput
        ? firstUserInput.text
        : "New Chat";

  return text.replace(/^\s+/, "").slice(0, 100);
}

function chatThreadToHistoryItem(thread: ChatThread): ChatHistoryItem {
  const now = new Date().toISOString();
  const updatedMode =
    thread.mode && !isLspChatMode(thread.mode) ? "AGENT" : thread.mode;

  return {
    ...thread,
    // Use thread title if available, otherwise truncated first user message
    title: thread.title || getFirstUserContentFromChat(thread.messages),
    createdAt: thread.createdAt ?? now,
    updatedAt: now,
    integration: thread.integration,
    currentMaximumContextTokens: thread.currentMaximumContextTokens,
    isTitleGenerated: thread.isTitleGenerated,
    automatic_patch: thread.automatic_patch,
    mode: updatedMode,
  };
}

function trajectoryToHistoryItem(data: TrajectoryData): ChatHistoryItem {
  const thread = trajectoryDataToChatThread(data);
  return {
    ...thread,
    createdAt: data.created_at,
    updatedAt: data.updated_at,
    title: data.title,
    isTitleGenerated: data.isTitleGenerated,
  };
}

export const historySlice = createSlice({
  name: "history",
  initialState,
  reducers: {
    saveChat: (state, action: PayloadAction<ChatThread>) => {
      if (action.payload.messages.length === 0) return state;
      const chat = chatThreadToHistoryItem(action.payload);
      const existing = state[chat.id];
      if (existing?.isTitleGenerated && !chat.isTitleGenerated) {
        chat.title = existing.title;
        chat.isTitleGenerated = true;
      }
      state[chat.id] = chat;

      const messages = Object.values(state);
      if (messages.length > 100) {
        const sorted = messages.sort((a, b) =>
          b.updatedAt.localeCompare(a.updatedAt),
        );
        return sorted.slice(0, 100).reduce(
          (acc, c) => ({ ...acc, [c.id]: c }),
          {},
        );
      }
    },

    hydrateHistory: (state, action: PayloadAction<TrajectoryData[]>) => {
      for (const data of action.payload) {
        state[data.id] = trajectoryToHistoryItem(data);
      }
    },

    markChatAsUnread: (state, action: PayloadAction<string>) => {
      if (action.payload in state) {
        state[action.payload].read = false;
      }
    },

    markChatAsRead: (state, action: PayloadAction<string>) => {
      if (action.payload in state) {
        state[action.payload].read = true;
      }
    },

    deleteChatById: (state, action: PayloadAction<string>) => {
      delete state[action.payload];
    },

    updateChatTitleById: (
      state,
      action: PayloadAction<{ chatId: string; newTitle: string }>,
    ) => {
      if (action.payload.chatId in state) {
        state[action.payload.chatId].title = action.payload.newTitle;
      }
    },

    clearHistory: () => {
      return {};
    },

    upsertToolCallIntoHistory: (
      state,
      action: PayloadAction<
        Parameters<typeof ideToolCallResponse>[0] & {
          replaceOnly?: boolean;
        }
      >,
    ) => {
      if (!(action.payload.chatId in state)) return;
      maybeAppendToolCallResultFromIdeToMessages(
        state[action.payload.chatId].messages,
        action.payload.toolCallId,
        action.payload.accepted,
        action.payload.replaceOnly,
      );
    },
  },
  selectors: {
    getChatById: (state, id: string): ChatHistoryItem | null => {
      if (!(id in state)) return null;
      return state[id];
    },

    getHistory: (state): ChatHistoryItem[] =>
      Object.values(state).sort((a, b) =>
        b.updatedAt.localeCompare(a.updatedAt),
      ),
  },
});

export const {
  saveChat,
  hydrateHistory,
  deleteChatById,
  markChatAsUnread,
  markChatAsRead,
  updateChatTitleById,
  clearHistory,
  upsertToolCallIntoHistory,
} = historySlice.actions;
export const { getChatById, getHistory } = historySlice.selectors;

async function persistToBackend(
  dispatch: AppDispatch,
  thread: ChatThread,
  existingCreatedAt?: string,
) {
  const data = chatThreadToTrajectoryData(thread, existingCreatedAt);
  dispatch(trajectoriesApi.endpoints.saveTrajectory.initiate(data));
}

export const historyMiddleware = createListenerMiddleware();
const startHistoryListening = historyMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startHistoryListening({
  actionCreator: doneStreaming,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();

    const runtime = state.chat.threads[action.payload.id];
    if (!runtime) return;
    const thread = runtime.thread;

    const existingChat = state.history[thread.id];
    const existingCreatedAt = existingChat?.createdAt;

    // Title generation is now handled by the backend
    listenerApi.dispatch(saveChat(thread));
    persistToBackend(listenerApi.dispatch, thread, existingCreatedAt);
  },
});

startHistoryListening({
  actionCreator: backUpMessages,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const runtime = state.chat.threads[action.payload.id];
    if (!runtime) return;
    const thread = runtime.thread;

    const existingChat = state.history[thread.id];
    const toSave = {
      ...thread,
      messages: action.payload.messages,
      project_name: thread.project_name ?? state.current_project.name,
    };
    listenerApi.dispatch(saveChat(toSave));
    persistToBackend(listenerApi.dispatch, toSave, existingChat?.createdAt);
  },
});

startHistoryListening({
  actionCreator: chatAskedQuestion,
  effect: (action, listenerApi) => {
    listenerApi.dispatch(markChatAsUnread(action.payload.id));
  },
});

startHistoryListening({
  actionCreator: restoreChat,
  effect: (action, listenerApi) => {
    const chat = listenerApi.getState().chat;
    const runtime = chat.threads[action.payload.id];
    if (runtime?.streaming) return;
    listenerApi.dispatch(markChatAsRead(action.payload.id));
  },
});

startHistoryListening({
  actionCreator: setChatMode,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const runtime = state.chat.threads[state.chat.current_thread_id];
    if (!runtime) return;
    const thread = runtime.thread;
    if (!(thread.id in state.history)) return;

    const existingChat = state.history[thread.id];
    const toSave = { ...thread, mode: action.payload };
    listenerApi.dispatch(saveChat(toSave));
    persistToBackend(listenerApi.dispatch, toSave, existingChat?.createdAt);
  },
});

startHistoryListening({
  actionCreator: deleteChatById,
  effect: (action, listenerApi) => {
    listenerApi.dispatch(
      trajectoriesApi.endpoints.deleteTrajectory.initiate(action.payload),
    );
  },
});
