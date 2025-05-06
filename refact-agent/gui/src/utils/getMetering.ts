import { Usage } from "../services/refact/chat";
import {
  AssistantMessage,
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
} from "../services/refact/types";

// TODO: cap cost should be in the messages:/
export function getTotalCostMeteringForMessages(messages: ChatMessages) {
  const assistantMessages = messages.filter(hasUsageAndPrice);
  if (assistantMessages.length === 0) return null;

  return assistantMessages.reduce<{
    metering_coins_prompt: number;
    metering_coins_generated: number;
    metering_coins_cache_creation: number;
    metering_coins_cache_read: number;
  }>(
    (acc, message) => {
      return {
        metering_coins_prompt:
          acc.metering_coins_prompt + message.metering_coins_prompt,
        metering_coins_generated:
          acc.metering_coins_generated + message.metering_coins_generated,
        metering_coins_cache_creation:
          acc.metering_coins_cache_creation +
          message.metering_coins_cache_creation,
        metering_coins_cache_read:
          acc.metering_coins_cache_read + message.metering_coins_cache_read,
      };
    },
    {
      metering_coins_prompt: 0,
      metering_coins_generated: 0,
      metering_coins_cache_creation: 0,
      metering_coins_cache_read: 0,
    },
  );
}

export function getTotalTokenMeteringForMessages(messages: ChatMessages) {
  const assistantMessages = messages.filter(hasUsageAndPrice);
  if (assistantMessages.length === 0) return null;

  return assistantMessages.reduce<{
    metering_prompt_tokens_n: number;
    metering_generated_tokens_n: number;
    metering_cache_creation_tokens_n: number;
    metering_cache_read_tokens_n: number;
  }>(
    (acc, message) => {
      const {
        metering_prompt_tokens_n,
        metering_generated_tokens_n,
        metering_cache_read_tokens_n,
        metering_cache_creation_tokens_n,
      } = message;
      return {
        metering_prompt_tokens_n:
          acc.metering_prompt_tokens_n + metering_prompt_tokens_n,
        metering_generated_tokens_n:
          acc.metering_generated_tokens_n + metering_generated_tokens_n,
        metering_cache_creation_tokens_n:
          acc.metering_cache_creation_tokens_n +
          metering_cache_creation_tokens_n,
        metering_cache_read_tokens_n:
          acc.metering_cache_read_tokens_n + metering_cache_read_tokens_n,
      };
    },
    {
      metering_prompt_tokens_n: 0,
      metering_generated_tokens_n: 0,
      metering_cache_creation_tokens_n: 0,
      metering_cache_read_tokens_n: 0,
    },
  );
}
function hasUsageAndPrice(message: ChatMessage): message is AssistantMessage & {
  usage: Usage & {
    completion_tokens: number;
    prompt_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
  };
  metering_coins_prompt: number;
  metering_coins_generated: number;
  metering_coins_cache_creation: number;
  metering_coins_cache_read: number;

  metering_prompt_tokens_n: number;
  metering_generated_tokens_n: number;
  metering_cache_creation_tokens_n: number;
  metering_cache_read_tokens_n: number;
} {
  if (!isAssistantMessage(message)) return false;
  if (!("usage" in message)) return false;
  if (!message.usage) return false;
  if (typeof message.usage.completion_tokens !== "number") return false;
  if (typeof message.usage.prompt_tokens !== "number") return false;
  if (typeof message.metering_coins_prompt !== "number") return false;
  if (typeof message.metering_coins_prompt !== "number") return false;
  if (typeof message.metering_coins_cache_creation !== "number") return false;
  if (typeof message.metering_coins_cache_read !== "number") return false;

  if (typeof message.metering_prompt_tokens_n !== "number") return false;
  if (typeof message.metering_generated_tokens_n !== "number") return false;
  if (typeof message.metering_cache_creation_tokens_n !== "number") {
    return false;
  }
  if (typeof message.metering_cache_read_tokens_n !== "number") return false;

  return true;
}
