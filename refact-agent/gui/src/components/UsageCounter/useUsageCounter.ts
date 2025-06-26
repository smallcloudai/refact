import { useMemo } from "react";
// import {
//   // selectIsStreaming,
//   // selectIsWaiting,
//   selectMessages,
// } from "../../features/Chat";
import {
  selectIsStreaming,
  selectIsWaiting,
} from "../../features/ThreadMessages";
import { useAppSelector /*useLastSentCompressionStop*/ } from "../../hooks";
import {
  calculateUsageInputTokens,
  mergeUsages,
} from "../../utils/calculateUsageInputTokens";
import { isAssistantMessage, isUsage, Usage } from "../../services/refact";
import { selectMessagesFromEndNode } from "../../features/ThreadMessages";

export function useUsageCounter() {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  // const compressionStop = useLastSentCompressionStop();
  // here, change to selectFromEndNode
  // const messages = useAppSelector(selectMessages);
  const messagesInBranch = useAppSelector(selectMessagesFromEndNode, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const assistantMessages = messagesInBranch.filter(isAssistantMessage);
  // const usages = assistantMessages.map((msg) => msg.ftm_usage);
  const usages = assistantMessages.reduce<Usage[]>((acc, cur) => {
    if (!isUsage(cur.ftm_usage)) return acc;
    return [...acc, cur.ftm_usage];
  }, []);
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

  const isOverflown = useMemo(
    () => {
      // if (compressionStop.strength === "low") return true;
      // if (compressionStop.strength === "medium") return true;
      // if (compressionStop.strength === "high") return true;
      return false;
    },
    [
      /*compressionStop.strength*/
    ],
  );

  const isWarning = useMemo(
    () => {
      // if (compressionStop.strength === "medium") return true;
      // if (compressionStop.strength === "high") return true;
      return false;
    },
    [
      /*compressionStop.strength*/
    ],
  );

  const shouldShow = useMemo(() => {
    return messagesInBranch.length > 0 && !isStreaming && !isWaiting;
  }, [messagesInBranch.length, isStreaming, isWaiting]);

  return {
    shouldShow,
    currentThreadUsage,
    totalInputTokens,
    isOverflown,
    isWarning,
  };
}
