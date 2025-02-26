import { useEffect, useMemo } from "react";
import {
  selectChatId,
  selectThreadMaximumTokens,
  selectThreadUsage,
  setIsNewChatCreationMandatory,
  setIsNewChatSuggested,
  // setIsNewChatSuggestionRejected,
} from "../../features/Chat";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";

export function useUsageCounter() {
  const dispatch = useAppDispatch();

  const chatId = useAppSelector(selectChatId);

  const currentThreadUsage = useAppSelector(selectThreadUsage);
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
    if (
      currentThreadMaximumContextTokens &&
      totalInputTokens > currentThreadMaximumContextTokens
    )
      return true;
    return false;
  }, [totalInputTokens, currentThreadMaximumContextTokens]);

  const isWarning = useMemo(() => {
    if (isOverflown) return false;
    if (
      currentThreadMaximumContextTokens &&
      totalInputTokens > currentThreadMaximumContextTokens * 0.75
    )
      return true;
    return false;
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

    actions.forEach((action) => dispatch(action));
  }, [dispatch, chatId, isWarning, isOverflown]);

  return {
    currentThreadUsage,
    totalInputTokens,
    isOverflown,
    isWarning,
  };
}
