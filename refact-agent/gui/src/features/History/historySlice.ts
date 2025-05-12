import {
  createSlice,
  PayloadAction,
  createListenerMiddleware,
} from "@reduxjs/toolkit";
import {
  backUpMessages,
  chatAskedQuestion,
  chatGenerateTitleThunk,
  ChatThread,
  doneStreaming,
  isLspChatMode,
  maybeAppendToolCallResultFromIdeToMessages,
  removeChatFromCache,
  restoreChat,
  setChatMode,
  SuggestedChat,
} from "../Chat/Thread";
import {
  isAssistantMessage,
  isChatGetTitleActionPayload,
  isUserMessage,
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
  const message = messages.find(isUserMessage);
  if (!message) return "New Chat";
  if (typeof message.content === "string") {
    return message.content.replace(/^\s+/, "");
  }

  const firstUserInput = message.content.find((message) => {
    if ("m_type" in message && message.m_type === "text") {
      return true;
    }
    if ("type" in message && message.type === "text") {
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

  return text.replace(/^\s+/, "");
}

export const historySlice = createSlice({
  name: "history",
  initialState,
  reducers: {
    saveChat: (state, action: PayloadAction<ChatThread>) => {
      if (action.payload.messages.length === 0) return state;
      const now = new Date().toISOString();

      const updatedMode =
        action.payload.mode && !isLspChatMode(action.payload.mode)
          ? "AGENT"
          : action.payload.mode;

      const chat: ChatHistoryItem = {
        ...action.payload,
        title: action.payload.title
          ? action.payload.title
          : getFirstUserContentFromChat(action.payload.messages),
        createdAt: action.payload.createdAt ?? now,
        updatedAt: now,
        // TODO: check if this integration may cause any issues
        integration: action.payload.integration,
        currentMaximumContextTokens: action.payload.currentMaximumContextTokens,
        isTitleGenerated: action.payload.isTitleGenerated,
        automatic_patch: action.payload.automatic_patch,
        mode: updatedMode,
      };

      const messageMap = {
        ...state,
      };
      messageMap[chat.id] = chat;

      const messages = Object.values(messageMap);
      if (messages.length <= 100) {
        return messageMap;
      }

      const sortedByLastUpdated = messages
        .slice(0)
        .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));

      const newHistory = sortedByLastUpdated.slice(0, 100);
      const nextState = newHistory.reduce(
        (acc, chat) => ({ ...acc, [chat.id]: chat }),
        {},
      );
      return nextState;
    },

    setTitleGenerationCompletionForChat: (
      state,
      action: PayloadAction<string>,
    ) => {
      const chatId = action.payload;
      state[chatId].isTitleGenerated = true;
    },

    markChatAsUnread: (state, action: PayloadAction<string>) => {
      const chatId = action.payload;
      state[chatId].read = false;
    },

    markChatAsRead: (state, action: PayloadAction<string>) => {
      const chatId = action.payload;
      state[chatId].read = true;
    },

    deleteChatById: (state, action: PayloadAction<string>) => {
      return Object.entries(state).reduce<Record<string, ChatHistoryItem>>(
        (acc, [key, value]) => {
          if (key === action.payload) return acc;
          return { ...acc, [key]: value };
        },
        {},
      );
    },
    updateChatTitleById: (
      state,
      action: PayloadAction<{ chatId: string; newTitle: string }>,
    ) => {
      state[action.payload.chatId].title = action.payload.newTitle;
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
  deleteChatById,
  markChatAsUnread,
  markChatAsRead,
  setTitleGenerationCompletionForChat,
  updateChatTitleById,
  clearHistory,
  upsertToolCallIntoHistory,
} = historySlice.actions;
export const { getChatById, getHistory } = historySlice.selectors;

// We could use this or reduce-reducers packages
export const historyMiddleware = createListenerMiddleware();
const startHistoryListening = historyMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startHistoryListening({
  actionCreator: doneStreaming,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const isTitleGenerationEnabled = state.chat.title_generation_enabled;

    const thread =
      action.payload.id in state.chat.cache
        ? state.chat.cache[action.payload.id]
        : state.chat.thread;

    const lastMessage = thread.messages.slice(-1)[0];
    const isTitleGenerated = thread.isTitleGenerated;
    // Checking for reliable chat pause
    if (
      thread.messages.length &&
      isAssistantMessage(lastMessage) &&
      !lastMessage.tool_calls
    ) {
      // Getting user message
      const firstUserMessage = thread.messages.find(isUserMessage);
      if (firstUserMessage) {
        // Checking if chat title is already generated, if not - generating it
        if (!isTitleGenerated && isTitleGenerationEnabled) {
          listenerApi
            .dispatch(
              chatGenerateTitleThunk({
                messages: [firstUserMessage],
                chatId: state.chat.thread.id,
              }),
            )
            .unwrap()
            .then((response) => {
              if (isChatGetTitleActionPayload(response)) {
                if (typeof response.title === "string") {
                  listenerApi.dispatch(
                    saveChat({
                      ...thread,
                      title: response.title,
                    }),
                  );
                  listenerApi.dispatch(
                    setTitleGenerationCompletionForChat(thread.id),
                  );
                }
              }
            })
            .catch(() => {
              // TODO: handle error in case if not generated, now returning user message as a title
              const title = getFirstUserContentFromChat([firstUserMessage]);
              listenerApi.dispatch(
                saveChat({
                  ...thread,
                  title: title,
                }),
              );
            });
        }
      }
    } else {
      // Probably chat was paused with uncalled tools
      listenerApi.dispatch(
        saveChat({
          ...thread,
        }),
      );
    }
    if (state.chat.thread.id === action.payload.id) {
      listenerApi.dispatch(saveChat(state.chat.thread));
    } else if (action.payload.id in state.chat.cache) {
      listenerApi.dispatch(saveChat(state.chat.cache[action.payload.id]));
      listenerApi.dispatch(removeChatFromCache({ id: action.payload.id }));
    }
  },
});

startHistoryListening({
  actionCreator: backUpMessages,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const thread = state.chat.thread;
    if (thread.id !== action.payload.id) return;
    const toSave = {
      ...thread,
      messages: action.payload.messages,
      project_name: thread.project_name ?? state.current_project.name,
    };
    listenerApi.dispatch(saveChat(toSave));
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
    if (chat.thread.id == action.payload.id && chat.streaming) return;
    if (action.payload.id in chat.cache) return;
    listenerApi.dispatch(markChatAsRead(action.payload.id));
  },
});

startHistoryListening({
  actionCreator: setChatMode,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const thread = state.chat.thread;
    if (!(thread.id in state.history)) return;

    const toSave = { ...thread, mode: action.payload };
    listenerApi.dispatch(saveChat(toSave));
  },
});

// TODO: add a listener for creating a new chat ?
