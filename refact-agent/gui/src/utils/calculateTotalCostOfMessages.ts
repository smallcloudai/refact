import { Usage } from "../services/refact/chat";
import {
  AssistantMessage,
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
} from "../services/refact/types";

export function calculateTotalCostOfMessages(messages: ChatMessages) {
  const assistantMessages = messages.filter(hasUsageAndPrice);
  if (assistantMessages.length === 0) return null;

  return assistantMessages.reduce<{
    cache_creation: number;
    cache_read: number;
  }>(
    (acc, message) => {
      const creation = calculateCost(
        message.usage.cache_creation_input_tokens,
        message.pp1000t_cache_creation,
      );

      const read = calculateCost(
        message.usage.cache_read_input_tokens,
        message.pp1000t_cache_read,
      );

      return {
        cache_creation: acc.cache_creation + creation,
        cache_read: acc.cache_read + read,
      };
    },
    { cache_creation: 0, cache_read: 0 },
  );
}
function hasUsageAndPrice(message: ChatMessage): message is AssistantMessage & {
  usage: Usage & {
    cache_creation_input_tokens: number;
    cache_read_input_tokens: number;
  };
  pp1000t_cache_creation: number;
  pp1000t_cache_read: number;
} {
  if (!isAssistantMessage(message)) return false;
  if (!("usage" in message)) return false;
  if (typeof message.usage?.cache_creation_input_tokens !== "number")
    return false;
  if (typeof message.usage.cache_read_input_tokens !== "number") return false;
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
