import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectLspPort, selectApiKey } from "../features/Config/configSlice";
import {
  sendChatCommand,
  type ChatCommandBase,
} from "../services/refact/chatCommands";

export function useSendChatCommand() {
  const port = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey);

  return useCallback(
    async (
      chatId: string,
      command: ChatCommandBase,
    ) => {
      await sendChatCommand(chatId, port, apiKey ?? undefined, command);
    },
    [port, apiKey],
  );
}
