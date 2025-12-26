import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectLspPort, selectApiKey } from "../features/Config/configSlice";
import {
  sendChatCommand,
  type ChatCommand,
} from "../services/refact/chatCommands";

export function useSendChatCommand() {
  const port = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey);

  return useCallback(
    async (
      chatId: string,
      command: Omit<ChatCommand, "client_request_id">,
    ) => {
      try {
        await sendChatCommand(chatId, port, apiKey || undefined, command);
      } catch (error) {
        console.error("[useSendChatCommand] Failed to send command:", error);
        throw error;
      }
    },
    [port, apiKey],
  );
}
