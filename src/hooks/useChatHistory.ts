import { useLocalStorage } from "usehooks-ts";
import { ChatMessages, isUserMessage } from "../services/refact";
import { ChatThread, EVENT_NAMES_TO_CHAT } from "../events";

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

  function saveChat(chat: ChatThread | ChatHistoryItem) {
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
  }

  function restoreChatFromHistory(chatId: string) {
    const chat = history.find((chat) => chat.id === chatId);
    if (chat) {
      window.postMessage(
        { type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT, payload: chat },
        "*",
      );
    }
  }

  function createNewChat() {
    window.postMessage({ type: EVENT_NAMES_TO_CHAT.NEW_CHAT }, "*");
  }

  function deleteChat(chatId: string) {
    const chats = history.filter((chat) => chat.id !== chatId);
    setHistory(chats);
  }

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
