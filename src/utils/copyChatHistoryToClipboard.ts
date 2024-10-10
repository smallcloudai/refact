import type { RootState } from "../app/store";

type CopyChatHistoryToClipboardResponse = {
  error?: string;
  success: boolean;
};

export const copyChatHistoryToClipboard = async (
  chatThread: RootState["history"]["thread"],
): Promise<CopyChatHistoryToClipboardResponse> => {
  const jsonString = JSON.stringify(chatThread, null, 2);

  try {
    await navigator.clipboard.writeText(jsonString);
    return { success: true };
  } catch (error) {
    if (error instanceof Error) {
      return { error: error.message, success: false };
    }
    return { error: "Unknown error", success: false };
  }
};
