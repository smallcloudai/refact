import type { RootState } from "../app/store";

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

function fallbackCopying(text: string) {
  const textArea = document.createElement("textarea");
  textArea.value = text;

  textArea.style.top = "0";
  textArea.style.left = "0";
  textArea.style.position = "fixed";

  document.body.appendChild(textArea);
  textArea.focus();
  textArea.select();

  document.execCommand("copy");
  document.body.removeChild(textArea);
}
