import { useEffect, useMemo } from "react";
import {
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThreadMaximumTokens,
  setIsNewChatCreationMandatory,
  setIsNewChatSuggested,
  // setIsNewChatSuggestionRejected,
} from "../../features/Chat";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  calculateUsageInputTokens,
  mergeUsages,
} from "../../utils/calculateUsageInputTokens";
import { isAssistantMessage } from "../../services/refact";

export function useUsageCounter() {
  const dispatch = useAppDispatch();

  const chatId = useAppSelector(selectChatId);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const messages = useAppSelector(selectMessages);
  const assistantMessages = messages.filter(isAssistantMessage);
  const usages = assistantMessages.map((msg) => msg.usage);
  const currentThreadUsage = mergeUsages(usages);

  const currentThreadMaximumContextTokens = useAppSelector(
    selectThreadMaximumTokens,
  );

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
    return !!(
      currentThreadMaximumContextTokens &&
      totalInputTokens > currentThreadMaximumContextTokens
    );
  }, [totalInputTokens, currentThreadMaximumContextTokens]);

  const isWarning = useMemo(() => {
    if (isOverflown) return false;
    return !!(
      currentThreadMaximumContextTokens &&
      totalInputTokens > currentThreadMaximumContextTokens * 0.75
    );
  }, [isOverflown, totalInputTokens, currentThreadMaximumContextTokens]);

  useEffect(() => {
    const actions = [
      setIsNewChatSuggested({
        chatId,
        value: isWarning || isOverflown,
      }),
      // setIsNewChatSuggestionRejected({
      //   chatId,
      //   value: false,
      // }),
      setIsNewChatCreationMandatory({
        chatId,
        value: isOverflown,
      }),
    ];
    // src/components/UsageCounter/UsageCounter.stories.tsx:58:9

    actions.forEach((action) => dispatch(action));
  }, [dispatch, chatId, isWarning, isOverflown]);

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
