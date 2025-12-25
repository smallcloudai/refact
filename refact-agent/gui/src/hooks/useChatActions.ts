/**
 * Chat Actions Hook
 *
 * Provides actions for the stateless chat system using the commands API.
 * All state comes from the SSE subscription - this hook only sends commands.
 */

import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectLspPort } from "../features/Config/configSlice";
import { selectChatId, selectThreadImages } from "../features/Chat/Thread/selectors";
import {
  sendUserMessage,
  retryFromIndex as retryFromIndexApi,
  updateChatParams,
  abortGeneration,
  respondToToolConfirmation,
  respondToToolConfirmations,
  type MessageContent,
} from "../services/refact/chatCommands";
import type { UserMessage } from "../services/refact/types";

export function useChatActions() {
  const port = useAppSelector(selectLspPort);
  const chatId = useAppSelector(selectChatId);
  const attachedImages = useAppSelector(selectThreadImages);

  /**
   * Build message content with attached images if any.
   */
  const buildMessageContent = useCallback(
    (text: string): MessageContent => {
      if (!attachedImages || attachedImages.length === 0) {
        return text;
      }

      const imageContents: Array<{ type: "image_url"; image_url: { url: string } }> = [];
      for (const img of attachedImages) {
        if (typeof img.content === "string") {
          imageContents.push({
            type: "image_url",
            image_url: { url: img.content },
          });
        }
      }

      if (imageContents.length === 0) {
        return text;
      }

      return [...imageContents, { type: "text" as const, text }];
    },
    [attachedImages],
  );

  /**
   * Submit a user message to the chat.
   */
  const submit = useCallback(
    async (question: string) => {
      if (!chatId || !port) return;

      const content = buildMessageContent(question);
      await sendUserMessage(chatId, content, port);
    },
    [chatId, port, buildMessageContent],
  );

  /**
   * Abort the current generation.
   */
  const abort = useCallback(async () => {
    if (!chatId || !port) return;
    await abortGeneration(chatId, port);
  }, [chatId, port]);

  /**
   * Update chat parameters (model, mode, etc.).
   */
  const setParams = useCallback(
    async (params: {
      model?: string;
      mode?: string;
      boost_reasoning?: boolean;
    }) => {
      if (!chatId || !port) return;
      await updateChatParams(chatId, params, port);
    },
    [chatId, port],
  );

  /**
   * Respond to tool confirmation (accept or reject).
   */
  const respondToTool = useCallback(
    async (toolCallId: string, accepted: boolean) => {
      if (!chatId || !port) return;
      await respondToToolConfirmation(chatId, toolCallId, accepted, port);
    },
    [chatId, port],
  );

  /**
   * Respond to multiple tool confirmations at once (batch).
   */
  const respondToTools = useCallback(
    async (decisions: Array<{ tool_call_id: string; accepted: boolean }>) => {
      if (!chatId || !port || decisions.length === 0) return;
      await respondToToolConfirmations(chatId, decisions, port);
    },
    [chatId, port],
  );

  /**
   * Retry from a specific message index.
   * This truncates all messages from the given index and sends a new user message.
   */
  const retryFromIndex = useCallback(
    async (index: number, newContent: UserMessage["content"]) => {
      if (!chatId || !port) return;

      // Convert content to string if it's an array
      let textContent: string;
      if (typeof newContent === "string") {
        textContent = newContent;
      } else if (Array.isArray(newContent)) {
        textContent = newContent
          .filter((c): c is { type: "text"; text: string } =>
            typeof c === "object" && c !== null && "type" in c && c.type === "text"
          )
          .map((c) => c.text)
          .join("\n");
      } else {
        textContent = "";
      }

      await retryFromIndexApi(chatId, index, textContent, port);
    },
    [chatId, port],
  );

  return {
    submit,
    abort,
    setParams,
    respondToTool,
    respondToTools,
    retryFromIndex,
  };
}

export default useChatActions;
