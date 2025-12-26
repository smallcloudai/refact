/**
 * Chat Actions Hook
 *
 * Provides actions for the stateless chat system using the commands API.
 * All state comes from the SSE subscription - this hook only sends commands.
 */

import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import { selectLspPort, selectApiKey } from "../features/Config/configSlice";
import { selectChatId, selectThreadImages } from "../features/Chat/Thread/selectors";
import { resetThreadImages } from "../features/Chat/Thread";
import {
  sendUserMessage,
  retryFromIndex as retryFromIndexApi,
  updateChatParams,
  abortGeneration,
  respondToToolConfirmation,
  respondToToolConfirmations,
  updateMessage as updateMessageApi,
  removeMessage as removeMessageApi,
  type MessageContent,
} from "../services/refact/chatCommands";
import type { UserMessage } from "../services/refact/types";

export function useChatActions() {
  const dispatch = useAppDispatch();
  const port = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey);
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

      if (text.trim().length === 0) {
        return imageContents;
      }

      return [{ type: "text" as const, text }, ...imageContents];
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
      await sendUserMessage(chatId, content, port, apiKey || undefined);
      dispatch(resetThreadImages({ id: chatId }));
    },
    [chatId, port, apiKey, buildMessageContent, dispatch],
  );

  /**
   * Abort the current generation.
   */
  const abort = useCallback(async () => {
    if (!chatId || !port) return;
    await abortGeneration(chatId, port, apiKey || undefined);
  }, [chatId, port, apiKey]);

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
      await updateChatParams(chatId, params, port, apiKey || undefined);
    },
    [chatId, port, apiKey],
  );

  /**
   * Respond to tool confirmation (accept or reject).
   */
  const respondToTool = useCallback(
    async (toolCallId: string, accepted: boolean) => {
      if (!chatId || !port) return;
      await respondToToolConfirmation(chatId, toolCallId, accepted, port, apiKey || undefined);
    },
    [chatId, port, apiKey],
  );

  /**
   * Respond to multiple tool confirmations at once (batch).
   */
  const respondToTools = useCallback(
    async (decisions: Array<{ tool_call_id: string; accepted: boolean }>) => {
      if (!chatId || !port || decisions.length === 0) return;
      await respondToToolConfirmations(chatId, decisions, port, apiKey || undefined);
    },
    [chatId, port, apiKey],
  );

  /**
   * Retry from a specific message index.
   * This truncates all messages from the given index and sends a new user message.
   */
  const retryFromIndex = useCallback(
    async (index: number, newContent: UserMessage["content"]) => {
      if (!chatId || !port) return;

      let content: MessageContent;
      if (typeof newContent === "string") {
        content = newContent;
      } else if (Array.isArray(newContent)) {
        type ContentItem = { type: "text"; text: string } | { type: "image_url"; image_url: { url: string } };
        const mapped: ContentItem[] = newContent.flatMap((item): ContentItem[] => {
          if (typeof item !== "object" || item === null) return [];
          if ("type" in item && item.type === "text" && "text" in item) {
            return [item as { type: "text"; text: string }];
          }
          if ("type" in item && item.type === "image_url" && "image_url" in item) {
            return [item as { type: "image_url"; image_url: { url: string } }];
          }
          if ("m_type" in item && "m_content" in item) {
            const m_type = (item as { m_type: unknown }).m_type;
            const m_content = (item as { m_content: unknown }).m_content;
            if (m_type === "text") {
              return [{ type: "text" as const, text: String(m_content ?? "") }];
            }
            if (typeof m_type === "string" && m_type.startsWith("image/")) {
              return [{
                type: "image_url" as const,
                image_url: { url: `data:${m_type};base64,${String(m_content ?? "")}` }
              }];
            }
          }
          return [];
        });
        content = mapped.length > 0 ? mapped : "";
      } else {
        content = "";
      }

      await retryFromIndexApi(chatId, index, content, port, apiKey || undefined);
    },
    [chatId, port, apiKey],
  );

  const updateMessage = useCallback(
    async (messageId: string, newContent: MessageContent, regenerate?: boolean) => {
      if (!chatId || !port) return;
      await updateMessageApi(chatId, messageId, newContent, port, apiKey || undefined, regenerate);
    },
    [chatId, port, apiKey],
  );

  const removeMessage = useCallback(
    async (messageId: string, regenerate?: boolean) => {
      if (!chatId || !port) return;
      await removeMessageApi(chatId, messageId, port, apiKey || undefined, regenerate);
    },
    [chatId, port, apiKey],
  );

  return {
    submit,
    abort,
    setParams,
    respondToTool,
    respondToTools,
    retryFromIndex,
    updateMessage,
    removeMessage,
  };
}

export default useChatActions;
