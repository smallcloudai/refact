import { SupportsReasoningStyle } from "../../../../../services/refact";
import { BEAUTIFUL_PROVIDER_NAMES } from "../../../constants";

export function isSupportsReasoningStyle(
  data: string | null,
): data is SupportsReasoningStyle {
  return (
    data === "openai" ||
    data === "anthropic" ||
    data === "deepseek" ||
    data === null
  );
}

export function extractHumanReadableReasoningType(
  reasoningType: string | null,
) {
  if (!isSupportsReasoningStyle(reasoningType)) return null;
  if (!reasoningType) return null;

  const maybeReadableReasoningType = BEAUTIFUL_PROVIDER_NAMES[reasoningType];

  return maybeReadableReasoningType
    ? maybeReadableReasoningType
    : reasoningType;
}
