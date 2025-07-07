import { isUsage, Usage } from "../services/refact/chat";
import { AssistantMessage, isAssistantMessage } from "../services/refact/types";

// TODO: cap cost should be in the messages and fix types
export function getTotalCostMeteringForMessages(messages: unknown[]) {
  const assistantMessages = messages.filter(hasUsageAndPrice);
  if (assistantMessages.length === 0) return null;

  return assistantMessages.reduce<{
    metering_coins_prompt: number;
    metering_coins_generated: number;
    metering_coins_cache_creation: number;
    metering_coins_cache_read: number;
  }>(
    (acc, message) => {
      // const metering_coins_prompt = message.ftm_usage.
      return {
        metering_coins_prompt:
          acc.metering_coins_prompt +
          (message.ftm_usage.tokens_prompt * message.ftm_usage.pp1000t_prompt) /
            1000, // message.metering_coins_prompt,
        metering_coins_generated:
          acc.metering_coins_generated +
          (message.ftm_usage.tokens_completion *
            message.ftm_usage.pp1000t_completion) /
            1000, // message.metering_coins_generated,
        metering_coins_cache_creation:
          acc.metering_coins_cache_creation +
          (message.ftm_usage.tokens_cache_creation *
            message.ftm_usage.pp1000t_cache_creation) /
            1000,
        // message.metering_coins_cache_creation,
        metering_coins_cache_read:
          acc.metering_coins_cache_read +
          (message.ftm_usage.tokens_cache_read *
            message.ftm_usage.pp1000t_cache_read) /
            1000, // message.metering_coins_cache_read,
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

// TODO: metering is gone :/
export function getTotalTokenMeteringForMessages(messages: unknown[]) {
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
        tokens_prompt,
        tokens_completion,
        tokens_cache_creation,
        tokens_cache_read,
      } = message.ftm_usage;
      return {
        metering_prompt_tokens_n: acc.metering_prompt_tokens_n + tokens_prompt,
        metering_generated_tokens_n:
          acc.metering_generated_tokens_n + tokens_completion,
        metering_cache_creation_tokens_n:
          acc.metering_cache_creation_tokens_n + tokens_cache_creation,
        metering_cache_read_tokens_n:
          acc.metering_cache_read_tokens_n + tokens_cache_read,
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
function hasUsageAndPrice(message: unknown): message is AssistantMessage & {
  ftm_usage: Usage;
} {
  if (!isAssistantMessage(message)) return false;
  if (!("ftm_usage" in message)) return false;
  if (!message.ftm_usage) return false;
  if (!isUsage(message.ftm_usage)) return false;
  // if (typeof message.ftm_usage.completion_tokens !== "number") return false;
  // if (typeof message.ftm_usage.prompt_tokens !== "number") return false;
  // if (typeof message.metering_coins_prompt !== "number") return false;
  // if (typeof message.metering_coins_prompt !== "number") return false;
  // if (typeof message.metering_coins_cache_creation !== "number") return false;
  // if (typeof message.metering_coins_cache_read !== "number") return false;

  // if (typeof message.metering_prompt_tokens_n !== "number") return false;
  // if (typeof message.metering_generated_tokens_n !== "number") return false;
  // if (typeof message.metering_cache_creation_tokens_n !== "number") {
  //   return false;
  // }
  // if (typeof message.metering_cache_read_tokens_n !== "number") return false;

  return true;
}
