import { Usage } from "../../services/refact";

export const USAGE_COUNTER_STUB_GPT: Usage = {
  completion_tokens: 30,
  prompt_tokens: 3391,
  total_tokens: 3421,
  completion_tokens_details: {
    accepted_prediction_tokens: 0,
    audio_tokens: 0,
    reasoning_tokens: 0,
    rejected_prediction_tokens: 0,
  },
  prompt_tokens_details: {
    audio_tokens: 0,
    cached_tokens: 3328,
  },
};

export const USAGE_COUNTER_STUB_ANTHROPIC: Usage = {
  completion_tokens: 142,
  prompt_tokens: 5,
  total_tokens: 147,
  completion_tokens_details: null,
  prompt_tokens_details: null,
  cache_creation_input_tokens: 3291,
  cache_read_input_tokens: 3608,
};
