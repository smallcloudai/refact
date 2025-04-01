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
      keys: ["prompt_tokens", "completion_tokens"],
      usage: undefined,
    });

    expect(result).toBe(0);
  });

  it("should sum specified numeric keys from usage", () => {
    const usage: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: null,
      prompt_tokens_details: null,
    };

    const result = calculateUsageInputTokens({
      keys: ["prompt_tokens", "completion_tokens"],
      usage,
    });

    expect(result).toBe(130);
  });

  it("should ignore non-numeric values", () => {
    const usage: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: {
        accepted_prediction_tokens: 20,
        audio_tokens: 0,
        reasoning_tokens: 10,
        rejected_prediction_tokens: 0,
      },
      prompt_tokens_details: null,
    };

    const result = calculateUsageInputTokens({
      keys: ["prompt_tokens", "completion_tokens", "completion_tokens_details"],
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
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: null,
      prompt_tokens_details: null,
    };

    const usage2: Usage = {
      completion_tokens: 20,
      prompt_tokens: 80,
      total_tokens: 100,
      completion_tokens_details: null,
      prompt_tokens_details: null,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result).toEqual({
      completion_tokens: 50,
      prompt_tokens: 180,
      total_tokens: 230,
      completion_tokens_details: null,
      prompt_tokens_details: null,
      cache_creation_input_tokens: 0,
      cache_read_input_tokens: 0,
    });
  });

  it("should correctly merge completion token details", () => {
    const usage1: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: {
        accepted_prediction_tokens: 10,
        audio_tokens: 0,
        reasoning_tokens: 20,
        rejected_prediction_tokens: 0,
      },
      prompt_tokens_details: null,
    };

    const usage2: Usage = {
      completion_tokens: 20,
      prompt_tokens: 80,
      total_tokens: 100,
      completion_tokens_details: {
        accepted_prediction_tokens: 5,
        audio_tokens: 0,
        reasoning_tokens: 10,
        rejected_prediction_tokens: 5,
      },
      prompt_tokens_details: null,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result?.completion_tokens_details).toEqual({
      accepted_prediction_tokens: 15,
      audio_tokens: 0,
      reasoning_tokens: 30,
      rejected_prediction_tokens: 5,
    });
  });

  it("should correctly merge prompt token details", () => {
    const usage1: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: null,
      prompt_tokens_details: {
        audio_tokens: 0,
        cached_tokens: 80,
      },
    };

    const usage2: Usage = {
      completion_tokens: 20,
      prompt_tokens: 80,
      total_tokens: 100,
      completion_tokens_details: null,
      prompt_tokens_details: {
        audio_tokens: 0,
        cached_tokens: 50,
      },
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result?.prompt_tokens_details).toEqual({
      audio_tokens: 0,
      cached_tokens: 130,
    });
  });

  it("should correctly merge cache-related fields", () => {
    const usage1: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: null,
      prompt_tokens_details: null,
      cache_creation_input_tokens: 50,
      cache_read_input_tokens: 20,
    };

    const usage2: Usage = {
      completion_tokens: 20,
      prompt_tokens: 80,
      total_tokens: 100,
      completion_tokens_details: null,
      prompt_tokens_details: null,
      cache_creation_input_tokens: 30,
      cache_read_input_tokens: 10,
    };

    const result = mergeUsages([usage1, usage2]);

    expect(result?.cache_creation_input_tokens).toBe(80);
    expect(result?.cache_read_input_tokens).toBe(30);
  });

  it("should handle complex merge scenario with multiple usage records", () => {
    const usage1: Usage = {
      completion_tokens: 30,
      prompt_tokens: 100,
      total_tokens: 130,
      completion_tokens_details: {
        accepted_prediction_tokens: 10,
        audio_tokens: 5,
        reasoning_tokens: 15,
        rejected_prediction_tokens: 0,
      },
      prompt_tokens_details: {
        audio_tokens: 0,
        cached_tokens: 80,
      },
      cache_creation_input_tokens: 50,
      cache_read_input_tokens: 20,
    };

    const usage2: Usage = {
      completion_tokens: 20,
      prompt_tokens: 80,
      total_tokens: 100,
      completion_tokens_details: {
        accepted_prediction_tokens: 5,
        audio_tokens: 0,
        reasoning_tokens: 10,
        rejected_prediction_tokens: 5,
      },
      prompt_tokens_details: {
        audio_tokens: 10,
        cached_tokens: 50,
      },
      cache_creation_input_tokens: 30,
      cache_read_input_tokens: 10,
    };

    // Add an undefined value to ensure it's properly filtered
    const result = mergeUsages([usage1, usage2, undefined]);

    expect(result).toEqual({
      completion_tokens: 50,
      prompt_tokens: 180,
      total_tokens: 230,
      completion_tokens_details: {
        accepted_prediction_tokens: 15,
        audio_tokens: 5,
        reasoning_tokens: 25,
        rejected_prediction_tokens: 5,
      },
      prompt_tokens_details: {
        audio_tokens: 10,
        cached_tokens: 130,
      },
      cache_creation_input_tokens: 80,
      cache_read_input_tokens: 30,
    });
  });

  it("should handle real-world usage examples", () => {
    const gptUsage: Usage = {
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

    const anthropicUsage: Usage = {
      completion_tokens: 142,
      prompt_tokens: 5,
      total_tokens: 147,
      completion_tokens_details: null,
      prompt_tokens_details: null,
      cache_creation_input_tokens: 3291,
      cache_read_input_tokens: 3608,
    };

    const result = mergeUsages([gptUsage, anthropicUsage]);

    expect(result).toEqual({
      completion_tokens: 172,
      prompt_tokens: 3396,
      total_tokens: 3568,
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
      cache_creation_input_tokens: 3291,
      cache_read_input_tokens: 3608,
    });
  });
});
