import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { isAssistantMessage } from "../events";
import { selectMessages } from "../features/Chat";
import {
  CompletionTokenDetails,
  PromptTokenDetails,
  type Usage,
} from "../services/refact/chat";
import { calculateUsageInputTokens } from "../utils/calculateUsageInputTokens";

const TOKEN_LIMIT = 200_000;
export function useTotalTokenUsage() {
  const messages = useAppSelector(selectMessages);

  const summedUsages = useMemo(() => {
    return messages.reduce<Usage | null>((acc, message) => {
      if (acc && isAssistantMessage(message) && message.usage) {
        return sumUsages(acc, message.usage);
      } else if (isAssistantMessage(message) && message.usage) {
        return message.usage;
      }
      return acc;
    }, null);
  }, [messages]);

  const tokens = useMemo(() => {
    if (!summedUsages) return 0;
    return calculateUsageInputTokens({
      keys: [
        "prompt_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "completion_tokens",
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

function addCompletionDetails(
  a: CompletionTokenDetails | null,
  b: CompletionTokenDetails | null,
): CompletionTokenDetails | null {
  if (!a && !b) return null;
  if (!a) return b;
  if (!b) return a;

  return {
    accepted_prediction_tokens:
      a.accepted_prediction_tokens + b.accepted_prediction_tokens,
    audio_tokens: a.audio_tokens + b.audio_tokens,
    reasoning_tokens: a.reasoning_tokens + b.reasoning_tokens,
    rejected_prediction_tokens:
      a.rejected_prediction_tokens + b.rejected_prediction_tokens,
  };
}

function addPromptTokenDetails(
  a: PromptTokenDetails | null,
  b: PromptTokenDetails | null,
): PromptTokenDetails | null {
  if (!a && !b) return null;
  if (!a) return b;
  if (!b) return a;

  return {
    audio_tokens: a.audio_tokens + b.audio_tokens,
    cached_tokens: a.cached_tokens + b.cached_tokens,
  };
}

function sumUsages(a: Usage, b: Usage): Usage {
  const completionDetails = addCompletionDetails(
    a.completion_tokens_details,
    b.completion_tokens_details,
  );
  const promptDetails = addPromptTokenDetails(
    a.prompt_tokens_details,
    b.prompt_tokens_details,
  );

  return {
    completion_tokens: a.completion_tokens + b.completion_tokens,
    prompt_tokens: a.prompt_tokens + b.prompt_tokens,
    total_tokens: a.total_tokens + b.total_tokens,
    completion_tokens_details: completionDetails,
    prompt_tokens_details: promptDetails,
    cache_creation_input_tokens:
      (a.cache_creation_input_tokens ?? 0) +
      (b.cache_creation_input_tokens ?? 0),
    cache_read_input_tokens:
      (a.cache_read_input_tokens ?? 0) + (b.cache_read_input_tokens ?? 0),
  };
}
