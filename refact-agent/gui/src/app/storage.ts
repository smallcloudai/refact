import type { WebStorage } from "redux-persist";
import {
  ChatHistoryItem,
  HistoryState,
} from "../features/History/historySlice";
import { parseOrElse } from "../utils";

type StoredState = {
  tipOfTheDay: string;
  tour: string;
  history: string;
};

function getOldest(history: HistoryState): ChatHistoryItem | null {
  const sorted = Object.values(history).sort((a, b) => {
    return new Date(a.updatedAt).getTime() - new Date(b.updatedAt).getTime();
  });
  const oldest = sorted[0] ?? null;
  return oldest;
}

function prune(key: string, stored: StoredState) {
  const history = parseOrElse<HistoryState>(stored.history, {});
  const oldest = getOldest(history);

  if (!oldest) return;
  const nextHistory = Object.values(history).reduce<HistoryState>(
    (acc, cur) => {
      if (cur.id === oldest.id) return acc;
      return { ...acc, [cur.id]: cur };
    },
    {},
  );
  const nextStorage = { ...stored, history: JSON.stringify(nextHistory) };
  try {
    const newHistory = JSON.stringify(nextStorage);
    localStorage.setItem(key, newHistory);
  } catch (e) {
    prune(key, nextStorage);
  }
}

function pruneHistory(key: string, item: string) {
  const storedString = item;
  if (!storedString) return;
  try {
    const stored = JSON.parse(storedString) as StoredState;
    prune(key, stored);
  } catch (e) {
    /* empty */
  }
}

function removeOldEntry(key: string) {
  if (localStorage.getItem(key)) {
    localStorage.removeItem(key);
  }
}

function cleanOldEntries() {
  removeOldEntry("tour");
  removeOldEntry("tipOfTheDay");
  removeOldEntry("chatHistory");
}

export function storage(): WebStorage {
  cleanOldEntries();
  return {
    getItem(key: string): Promise<string | null> {
      return new Promise((resolve, _reject) => {
        resolve(localStorage.getItem(key));
      });
    },
    setItem(key: string, item: string): Promise<void> {
      return new Promise((resolve, _reject) => {
        try {
          localStorage.setItem(key, item);
        } catch {
          pruneHistory(key, item);
        }
        resolve();
      });
    },
    removeItem(key: string): Promise<void> {
      return new Promise((resolve, _reject) => {
        localStorage.removeItem(key);
        resolve();
      });
    },
  };
}
