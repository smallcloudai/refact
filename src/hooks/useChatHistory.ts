import { useState, useEffect, useCallback } from "react";
import { useLocalStorage } from "usehooks-ts";
import { ChatMessages, isUserMessage } from "../services/refact";
import {
  ChatThread,
  EVENT_NAMES_TO_CHAT,
  RestoreChat,
  CreateNewChatThread,
  isReadyMessage,
} from "../events";

export type ChatHistoryItem = {
  id: string;
  createdAt: string; // Date
  lastUpdated: string; // Date
  messages: ChatMessages;
  title: string;
  model: string;
};

export function useChatHistory() {
  const [history, setHistory] = useLocalStorage<ChatHistoryItem[]>(
    "chatHistory",
    [],
  );

  const [currentChatId, setCurrentChatId] = useState<string>("");

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isReadyMessage(event.data)) {
        const { payload } = event.data;
        setCurrentChatId(payload.id);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, []);

  const saveChat = useCallback(
    (chat: ChatThread | ChatHistoryItem) => {
      const maybeChat = history.find((item) => item.id === chat.id);
      const now = new Date().toISOString();

      if (maybeChat) {
        maybeChat.lastUpdated = now;
        maybeChat.messages = chat.messages;
        const chats = history
          .filter((item) => item.id !== chat.id)
          .concat(maybeChat);

        setHistory(chats);
      } else {
        const firstUserMessage = chat.messages.find((message) =>
          isUserMessage(message),
        );
        let title = "New Chat";

        if (firstUserMessage && isUserMessage(firstUserMessage)) {
          title = firstUserMessage[1].replace(/^\W*/, "");
        }

        const newChat: ChatHistoryItem = {
          id: chat.id,
          messages: chat.messages,
          title,
          model: chat.model,
          createdAt: now,
          lastUpdated: now,
        };
        const chats = history.concat(newChat);
        setHistory(chats);
      }
    },
    [history, setHistory],
  );

  const restoreChatFromHistory = useCallback(
    (chatId: string) => {
      const chat = history.find((chat) => chat.id === chatId);
      if (chat) {
        const message: RestoreChat = {
          type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
          payload: {
            id: currentChatId,
            chat,
          },
        };
        window.postMessage(message, "*");
      }
    },
    [currentChatId, history],
  );

  const createNewChat = useCallback(() => {
    const message: CreateNewChatThread = {
      type: EVENT_NAMES_TO_CHAT.NEW_CHAT,
      payload: {
        id: currentChatId,
      },
    };
    window.postMessage(message, "*");
  }, [currentChatId]);

  const deleteChat = useCallback(
    (chatId: string) => {
      const chats = history.filter((chat) => chat.id !== chatId);
      setHistory(chats);
    },
    [history, setHistory],
  );

  const sortedHistory = history.slice().sort((a, b) => {
    return a.createdAt < b.createdAt ? 1 : -1;
  });

  return {
    history: sortedHistory,
    setHistory,
    saveChat,
    restoreChatFromHistory,
    createNewChat,
    deleteChat,
  };
}
