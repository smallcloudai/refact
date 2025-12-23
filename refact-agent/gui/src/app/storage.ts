import type { WebStorage } from "redux-persist";

function removeOldEntry(key: string) {
  if (
    typeof localStorage !== "undefined" &&
    typeof localStorage.getItem === "function" &&
    localStorage.getItem(key)
  ) {
    localStorage.removeItem(key);
  }
}

function cleanOldEntries() {
  if (typeof localStorage === "undefined") return;
  removeOldEntry("tour");
  removeOldEntry("tipOfTheDay");
  removeOldEntry("chatHistory");
}

export function storage(): WebStorage {
  cleanOldEntries();
  return {
    getItem(key: string): Promise<string | null> {
      return new Promise((resolve) => {
        resolve(localStorage.getItem(key));
      });
    },
    setItem(key: string, item: string): Promise<void> {
      return new Promise((resolve) => {
        try {
          localStorage.setItem(key, item);
        } catch {
          // Storage quota exceeded, ignore
        }
        resolve();
      });
    },
    removeItem(key: string): Promise<void> {
      return new Promise((resolve) => {
        localStorage.removeItem(key);
        resolve();
      });
    },
  };
}
