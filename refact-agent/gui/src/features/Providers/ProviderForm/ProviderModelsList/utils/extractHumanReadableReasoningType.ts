import { SupportsReasoningStyle } from "../../../../../services/refact";
import { BEAUTIFUL_PROVIDER_NAMES } from "../../../constants";

export function extractHumanReadableReasoningType(
  reasoningType: SupportsReasoningStyle,
) {
  if (!reasoningType) return null;
  const maybeReadableReasoningType = BEAUTIFUL_PROVIDER_NAMES[reasoningType];

  return maybeReadableReasoningType
    ? maybeReadableReasoningType
    : reasoningType;
}
