import { SupportsReasoningStyle } from "../../../../../services/refact";

export function extractHumanReadableReasoningType(
  reasoningType: SupportsReasoningStyle,
) {
  if (reasoningType === "openai") return "OpenAI";
  if (reasoningType === "anthropic") return "Anthropic";
  if (reasoningType === "deepseek") return "DeepSeek";
  return reasoningType;
}
