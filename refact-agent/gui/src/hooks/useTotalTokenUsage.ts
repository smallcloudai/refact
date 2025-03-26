import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { isAssistantMessage } from "../events";
import { selectMessages } from "../features/Chat";
import { type Usage } from "../services/refact/chat";
import {
  calculateUsageInputTokens,
  mergeUsages,
} from "../utils/calculateUsageInputTokens";

const TOKEN_LIMIT = 200_000;
// TODO: maybe remove this
export function useTotalTokenUsage() {
  const messages = useAppSelector(selectMessages);

  const summedUsages = useMemo(() => {
    const usages = messages.reduce<Usage[]>((acc, message) => {
      if (isAssistantMessage(message) && message.usage) {
        return [...acc, message.usage];
      }
      return acc;
    }, []);
    return mergeUsages(usages);
  }, [messages]);

  const tokens = useMemo(() => {
    if (!summedUsages) return 0;
    return calculateUsageInputTokens({
      keys: [
        "prompt_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
      ],
      usage: summedUsages,
    });
  }, [summedUsages]);

  const limitReached = useMemo(() => {
    return tokens >= TOKEN_LIMIT;
  }, [tokens]);

  return {
    summedUsages,
    tokens,
    limitReached,
    limit: TOKEN_LIMIT,
  };
}
