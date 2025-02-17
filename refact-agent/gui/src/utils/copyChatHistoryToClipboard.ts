import type { RootState } from "../app/store";
import { fallbackCopying } from "./fallbackCopying";

export const copyChatHistoryToClipboard = async (
  chatThread: RootState["history"]["thread"],
): Promise<void> => {
  const jsonString = JSON.stringify(chatThread, null, 2);

  try {
    await window.navigator.clipboard.writeText(jsonString);
  } catch {
    fallbackCopying(jsonString);
  }
};
