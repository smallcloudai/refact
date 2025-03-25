import { useMemo } from "react";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectLastSentCompression,
} from "../../features/Chat";
import { useAppSelector } from "../../hooks";
import {
  calculateUsageInputTokens,
  mergeUsages,
} from "../../utils/calculateUsageInputTokens";
import { isAssistantMessage } from "../../services/refact";

export function useUsageCounter() {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  const messages = useAppSelector(selectMessages);
  const assistantMessages = messages.filter(isAssistantMessage);
  const usages = assistantMessages.map((msg) => msg.usage);
  const currentThreadUsage = mergeUsages(usages);

  const totalInputTokens = useMemo(() => {
    return calculateUsageInputTokens({
      usage: currentThreadUsage,
      keys: [
        "prompt_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
      ],
    });
  }, [currentThreadUsage]);

  const isOverflown = useMemo(() => {
    if (lastSentCompression === "low") return true;
    if (lastSentCompression === "medium") return true;
    if (lastSentCompression === "high") return true;
    return false;
  }, [lastSentCompression]);

  const isWarning = useMemo(() => {
    if (lastSentCompression === "medium") return true;
    if (lastSentCompression === "high") return true;
    return false;
  }, [lastSentCompression]);

  const shouldShow = useMemo(() => {
    return messages.length > 0 && !isStreaming && !isWaiting;
  }, [messages, isStreaming, isWaiting]);

  return {
    shouldShow,
    currentThreadUsage,
    totalInputTokens,
    isOverflown,
    isWarning,
  };
}
