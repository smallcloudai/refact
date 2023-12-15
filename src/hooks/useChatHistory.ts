import { useLocalStorage } from "usehooks-ts";
import { ChatMessages } from "../services/refact";
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

  // TODO: add model
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
      const firstMessage = chat.messages.find(
        (message) => message[0] === "user",
      );
      const title = firstMessage ? firstMessage[1] : "New Chat";

      const newChat: ChatHistoryItem = {
        id: chat.id,
        messages: chat.messages,
        title: title,
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
    if(chat){
      window.postMessage({ type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT, payload: chat}, "*");
    }
  }

  function createNewChat() {
    window.postMessage({type: EVENT_NAMES_TO_CHAT.NEW_CHAT}, "*");
  }

  const sortedHistory = history.slice().sort((a, b) => {
    return a.createdAt < b.createdAt? 1 : -1;
  })

  return { history: sortedHistory, setHistory, saveChat, restoreChatFromHistory, createNewChat };
}
