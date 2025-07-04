import { describe, expect, it } from "vitest";
import {
  Usage,
  PromptTokenDetails,
  CompletionTokenDetails,
} from "../services/refact";
import {
  calculateUsageInputTokens,
  sumValues,
  mergeCompletionTokensDetails,
  mergePromptTokensDetails,
  mergeUsages,
} from "./calculateUsageInputTokens";

describe("calculateUsageInputTokens", () => {
  it("should return 0 for undefined usage", () => {
    const result = calculateUsageInputTokens({
      keys: ["tokens_prompt", "tokens_completion"],
      usage: undefined,
    });

    expect(result).toBe(0);
  });

  it("should sum specified numeric keys from usage", () => {
    const usage: Usage = {
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const result = calculateUsageInputTokens({
      keys: ["tokens_prompt", "tokens_completion"],
      usage,
    });

    expect(result).toBe(130);
  });

  it("should ignore non-numeric values", () => {
    const usage: Usage = {
      // completion_tokens: 30,
      // prompt_tokens: 100,
      // total_tokens: 130,
      // completion_tokens_details: {
      //   accepted_prediction_tokens: 20,
      //   audio_tokens: 0,
      //   reasoning_tokens: 10,
      //   rejected_prediction_tokens: 0,
      // },
      // prompt_tokens_details: null,
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 10,
      pp1000t_completion_reasoning: 0,
    };

    const result = calculateUsageInputTokens({
      keys: [
        "tokens_prompt",
        "tokens_completion",
        "tokens_completion_reasoning",
      ],
      usage,
    });

    expect(result).toBe(130); // Should only sum prompt_tokens and completion_tokens
  });
});

describe("sumValues", () => {
  it("should handle null and undefined values", () => {
    expect(sumValues(null, null)).toBe(0);
    expect(sumValues(undefined, undefined)).toBe(0);
    expect(sumValues(null, 5)).toBe(5);
    expect(sumValues(10, undefined)).toBe(10);
  });

  it("should sum two numbers", () => {
    expect(sumValues(5, 10)).toBe(15);
    expect(sumValues(0, 10)).toBe(10);
    expect(sumValues(-5, 10)).toBe(5);
  });
});

describe("mergeCompletionTokensDetails", () => {
  it("should return null when both inputs are null/undefined", () => {
    expect(mergeCompletionTokensDetails(null, null)).toBeNull();
    expect(mergeCompletionTokensDetails(undefined, null)).toBeNull();
    expect(mergeCompletionTokensDetails(null, undefined)).toBeNull();
    expect(mergeCompletionTokensDetails(undefined, undefined)).toBeNull();
  });

  it("should return the non-null input when one input is null/undefined", () => {
    const details: CompletionTokenDetails = {
      accepted_prediction_tokens: 10,
      audio_tokens: 5,
      reasoning_tokens: 20,
      rejected_prediction_tokens: 2,
    };

    expect(mergeCompletionTokensDetails(details, null)).toEqual(details);
    expect(mergeCompletionTokensDetails(null, details)).toEqual(details);
  });

  it("should merge two completion token details objects", () => {
    const details1: CompletionTokenDetails = {
      accepted_prediction_tokens: 10,
      audio_tokens: 5,
      reasoning_tokens: 20,
      rejected_prediction_tokens: 2,
    };

    const details2: CompletionTokenDetails = {
      accepted_prediction_tokens: 15,
      audio_tokens: 0,
      reasoning_tokens: 10,
      rejected_prediction_tokens: 3,
    };

    const expected: CompletionTokenDetails = {
      accepted_prediction_tokens: 25,
      audio_tokens: 5,
      reasoning_tokens: 30,
      rejected_prediction_tokens: 5,
    };

    expect(mergeCompletionTokensDetails(details1, details2)).toEqual(expected);
  });
});

describe("mergePromptTokensDetails", () => {
  it("should return null when both inputs are null/undefined", () => {
    expect(mergePromptTokensDetails(null, null)).toBeNull();
    expect(mergePromptTokensDetails(undefined, null)).toBeNull();
    expect(mergePromptTokensDetails(null, undefined)).toBeNull();
    expect(mergePromptTokensDetails(undefined, undefined)).toBeNull();
  });

  it("should return the non-null input when one input is null/undefined", () => {
    const details: PromptTokenDetails = {
      audio_tokens: 5,
      cached_tokens: 100,
    };

    expect(mergePromptTokensDetails(details, null)).toEqual(details);
    expect(mergePromptTokensDetails(null, details)).toEqual(details);
  });

  it("should merge two prompt token details objects", () => {
    const details1: PromptTokenDetails = {
      audio_tokens: 5,
      cached_tokens: 100,
    };

    const details2: PromptTokenDetails = {
      audio_tokens: 10,
      cached_tokens: 200,
    };

    const expected: PromptTokenDetails = {
      audio_tokens: 15,
      cached_tokens: 300,
    };

    expect(mergePromptTokensDetails(details1, details2)).toEqual(expected);
  });
});

describe("mergeUsages", () => {
  it("should return undefined for empty array", () => {
    expect(mergeUsages([])).toBeUndefined();
  });

  it("should return undefined when all array elements are undefined", () => {
    expect(mergeUsages([undefined, undefined])).toBeUndefined();
  });

  it("should correctly merge basic usage fields", () => {
    const usage1: Usage = {
      // completion_tokens: 30,
      // prompt_tokens: 100,
      // total_tokens: 130,
      // completion_tokens_details: null,
      // prompt_tokens_details: null,
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const usage2: Usage = {
      // completion_tokens: 20,
      // prompt_tokens: 80,
      // total_tokens: 100,
      // completion_tokens_details: null,
      // prompt_tokens_details: null,
      coins: 0,
      tokens_prompt: 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result).toEqual({
      coins: 0,
      tokens_prompt: 100 + 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30 + 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    });
  });

  it("should correctly merge completion token details", () => {
    const usage1: Usage = {
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 20,
      pp1000t_completion_reasoning: 0,
    };

    const usage2: Usage = {
      coins: 0,
      tokens_prompt: 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30 + 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 10,
      pp1000t_completion_reasoning: 0,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result).toEqual({
      coins: 0,
      tokens_prompt: 100 + 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 20 + 10,
      pp1000t_completion_reasoning: 0,
    });
  });

  it("should correctly merge cache-related fields", () => {
    const usage1: Usage = {
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 20,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 50,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const usage2: Usage = {
      coins: 0,
      tokens_prompt: 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 10,
      tokens_completion: 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 30,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result?.tokens_cache_creation).toBe(80);
    expect(result?.tokens_cache_read).toBe(30);
  });

  it("should handle complex merge scenario with multiple usage records", () => {
    const usage1: Usage = {
      coins: 0,
      tokens_prompt: 100,
      pp1000t_prompt: 0,
      tokens_cache_read: 20,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 50,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 5,
      tokens_completion_reasoning: 15,
      pp1000t_completion_reasoning: 0,
    };

    const usage2: Usage = {
      coins: 0,
      tokens_prompt: 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 10,
      tokens_completion: 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 30,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 10,
      pp1000t_completion_reasoning: 0,
    };

    // Add an undefined value to ensure it's properly filtered
    const result = mergeUsages([usage1, usage2, undefined]);

    expect(result).toEqual({
      coins: 0,
      tokens_prompt: 100 + 80,
      pp1000t_prompt: 0,
      tokens_cache_read: 20 + 10,
      tokens_completion: 30 + 20,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 50 + 30,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 5,
      tokens_completion_reasoning: 15 + 10,
      pp1000t_completion_reasoning: 0,
    });
  });

  it("should handle real-world usage examples", () => {
    const gptUsage: Usage = {
      coins: 0,
      tokens_prompt: 3391,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 30,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 0,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const anthropicUsage: Usage = {
      coins: 0,
      tokens_prompt: 5,
      pp1000t_prompt: 0,
      tokens_cache_read: 0,
      tokens_completion: 142,
      pp1000t_cache_read: 3608,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 3291,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 0,
      tokens_completion_reasoning: 0,
      pp1000t_completion_reasoning: 0,
    };

    const result = mergeUsages([gptUsage, anthropicUsage]);

    expect(result).toEqual({
      coins: 0,
      tokens_prompt: 3396,
      pp1000t_prompt: 0,
      tokens_cache_read: 3608,
      tokens_completion: 172,
      pp1000t_cache_read: 0,
      pp1000t_completion: 0,
      tokens_prompt_text: 0,
      tokens_prompt_audio: 0,
      tokens_prompt_image: 0,
      tokens_prompt_cached: 0,
      tokens_cache_creation: 3291,
      pp1000t_cache_creation: 0,
      tokens_completion_text: 0,
      tokens_completion_audio: 5,
      tokens_completion_reasoning: 15,
      pp1000t_completion_reasoning: 0,
    });
  });
});
