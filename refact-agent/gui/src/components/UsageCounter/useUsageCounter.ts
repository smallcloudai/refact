import { useMemo } from "react";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  // selectLastSentCompression,
} from "../../features/Chat";
import { useAppSelector, useLastSentCompressionStop } from "../../hooks";
import {
  calculateUsageInputTokens,
  mergeUsages,
} from "../../utils/calculateUsageInputTokens";
import { isAssistantMessage } from "../../services/refact";

export function useUsageCounter() {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const compressionStop = useLastSentCompressionStop();
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
    if (!compressionStop.stopped) return false;
    if (compressionStop.strength === "low") return true;
    if (compressionStop.strength === "medium") return true;
    if (compressionStop.strength === "high") return true;
    return false;
  }, [compressionStop.stopped, compressionStop.strength]);

  const isWarning = useMemo(() => {
    if (!compressionStop.stopped) return false;
    if (compressionStop.strength === "medium") return true;
    if (compressionStop.strength === "high") return true;
    return false;
  }, [compressionStop.stopped, compressionStop.strength]);

  const shouldShow = useMemo(() => {
    return messages.length > 0 && !isStreaming && !isWaiting;
  }, [messages.length, isStreaming, isWaiting]);

  return {
    shouldShow,
    currentThreadUsage,
    totalInputTokens,
    isOverflown,
    isWarning,
  };
}
