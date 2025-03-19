import { Usage } from "../services/refact";

export const calculateUsageInputTokens = ({
  keys,
  usage,
}: {
  keys: (keyof Usage)[];
  usage?: Usage;
}): number => {
  return keys.reduce((acc, key) => {
    if (!(usage && key in usage)) return acc;
    const value = usage[key];
    return acc + (typeof value === "number" ? value : 0);
  }, 0);
};

export function mergeUsages(usages: (Usage | undefined)[]): Usage | undefined {
  return usages.reduce<Usage | undefined>(
    (acc, usage) => {
      if (!usage) return acc;
      return {
        completion_tokens:
          (acc?.completion_tokens ?? 0) + (usage.completion_tokens || 0),
        prompt_tokens: (acc?.prompt_tokens ?? 0) + (usage.prompt_tokens || 0),
        total_tokens: (acc?.total_tokens ?? 0) + (usage.total_tokens || 0),

        // Merge completion_tokens_details if they exist
        completion_tokens_details:
          acc?.completion_tokens_details && usage.completion_tokens_details
            ? {
                accepted_prediction_tokens:
                  acc.completion_tokens_details.accepted_prediction_tokens +
                  usage.completion_tokens_details.accepted_prediction_tokens,
                audio_tokens:
                  acc.completion_tokens_details.audio_tokens +
                  usage.completion_tokens_details.audio_tokens,
                reasoning_tokens:
                  acc.completion_tokens_details.reasoning_tokens +
                  usage.completion_tokens_details.reasoning_tokens,
                rejected_prediction_tokens:
                  (acc.completion_tokens_details.rejected_prediction_tokens ||
                    0) +
                  (usage.completion_tokens_details.rejected_prediction_tokens ||
                    0),
              }
            : acc?.completion_tokens_details ??
              usage.completion_tokens_details ??
              null,

        // Merge prompt_tokens_details if they exist
        prompt_tokens_details:
          acc?.prompt_tokens_details && usage.prompt_tokens_details
            ? {
                audio_tokens:
                  (acc.prompt_tokens_details.audio_tokens || 0) +
                  (usage.prompt_tokens_details.audio_tokens || 0),
                cached_tokens:
                  (acc.prompt_tokens_details.cached_tokens || 0) +
                  (usage.prompt_tokens_details.cached_tokens || 0),
              }
            : acc?.prompt_tokens_details ?? usage.prompt_tokens_details ?? null,

        // Sum cache-related fields if they exist
        cache_creation_input_tokens:
          (acc?.cache_creation_input_tokens ?? 0) +
          (usage.cache_creation_input_tokens
            ? usage.cache_creation_input_tokens
            : 0),
        cache_read_input_tokens:
          (acc?.cache_read_input_tokens ?? 0) +
          (usage.cache_read_input_tokens ?? 0),
      };
    },
    {
      completion_tokens: 0,
      prompt_tokens: 0,
      total_tokens: 0,
      completion_tokens_details: null,
      prompt_tokens_details: null,
      cache_creation_input_tokens: 0,
      cache_read_input_tokens: 0,
    },
  );
}
