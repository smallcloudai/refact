import { CapCost } from "../services/refact/caps";
import { Usage } from "../services/refact/chat";
import {
  AssistantMessage,
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
} from "../services/refact/types";

// TODO: cap cost should be in the messages:/
export function calculateTotalCostOfMessages(
  messages: ChatMessages,
  capCost: CapCost,
) {
  const assistantMessages = messages.filter(hasUsageAndPrice);
  if (assistantMessages.length === 0) return null;

  return assistantMessages.reduce<{
    cache_creation: number;
    cache_read: number;
    prompt: number;
    generated: number;
  }>(
    (acc, message) => {
      const creation = calculateCost(
        message.usage.cache_creation_input_tokens ?? 0,
        message.pp1000t_cache_creation,
      );

      const read = calculateCost(
        message.usage.cache_read_input_tokens ?? 0,
        message.pp1000t_cache_read,
      );

      // TODO: units don't match up
      const prompt =
        (message.usage.prompt_tokens * capCost.generated) / 1000000;

      const generated =
        (message.usage.completion_tokens * capCost.generated) / 1000000;

      return {
        cache_creation: acc.cache_creation + creation,
        cache_read: acc.cache_read + read,
        prompt: acc.prompt + prompt,
        generated: acc.generated + generated,
      };
    },
    { cache_creation: 0, cache_read: 0, prompt: 0, generated: 0 },
  );
}
function hasUsageAndPrice(message: ChatMessage): message is AssistantMessage & {
  usage: Usage & {
    completion_tokens: number;
    prompt_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
  };
  pp1000t_cache_creation: number;
  pp1000t_cache_read: number;
} {
  if (!isAssistantMessage(message)) return false;
  if (!("usage" in message)) return false;
  if (!message.usage) return false;
  if (typeof message.usage.completion_tokens !== "number") return false;
  if (typeof message.usage.prompt_tokens !== "number") return false;
  // if (typeof message.usage?.cache_creation_input_tokens !== "number")
  //   return false;
  // if (typeof message.usage.cache_read_input_tokens !== "number") return false;
  if (typeof message.pp1000t_cache_creation !== "number") return false;
  if (typeof message.pp1000t_cache_read !== "number") return false;

  return true;
}

function calculateCost(tokens: number, costPerThousand: number): number {
  const costPerToken = costPerThousand / 1000;
  const totalCost = tokens * costPerToken;
  const mtok = totalCost / 1_000_000;
  return mtok;
}
