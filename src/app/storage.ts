import type { WebStorage } from "redux-persist";
import type { TipOfTheDayState } from "../features/TipOfTheDay";
import type { TourState } from "../features/Tour";
import {
  ChatHistoryItem,
  HistoryState,
} from "../features/History/historySlice";

type StoredState = {
  tipOfTheDay: TipOfTheDayState;
  tour: TourState;
  history: HistoryState;
};

function getOldest(history: HistoryState): ChatHistoryItem | null {
  const sorted = Object.values(history).sort((a, b) => {
    return new Date(a.updatedAt).getTime() - new Date(b.updatedAt).getTime();
  });
  const oldest = sorted[0] ?? null;
  return oldest;
}

function prune(key: string, stored: StoredState) {
  const oldest = getOldest(stored.history);
  if (!oldest) return;
  const history = Object.values(stored.history).reduce<HistoryState>(
    (acc, cur) => {
      if (cur.id === oldest.id) return acc;
      return { ...acc, [cur.id]: cur };
    },
    {},
  );
  const nextStorage = { ...stored, history };
  try {
    const newHistory = JSON.stringify({ ...stored, history });
    localStorage.setItem(key, newHistory);
  } catch (e) {
    prune(key, nextStorage);
  }
}

function pruneHistory(key: string) {
  const storedString = localStorage.getItem(key);
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
          pruneHistory(key);
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
