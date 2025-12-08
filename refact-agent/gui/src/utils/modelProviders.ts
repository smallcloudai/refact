/**
 * Extract provider name from model name (e.g., "refact/gpt-4o" -> "openai")
 */
export function extractProvider(modelName: string): string {
  const name = modelName.replace(/^refact\//, "").toLowerCase();

  if (name.includes("gpt") || name.includes("o1") || name.includes("o3")) {
    return "openai";
  }
  if (name.includes("claude")) {
    return "anthropic";
  }
  if (name.includes("gemini") || name.includes("palm")) {
    return "google";
  }
  if (name.includes("grok")) {
    return "x.ai";
  }
  if (name.includes("llama")) {
    return "meta";
  }
  if (name.includes("mistral") || name.includes("mixtral")) {
    return "mistral";
  }
  if (name.includes("qwen")) {
    return "alibaba";
  }
  if (name.includes("deepseek")) {
    return "deepseek";
  }

  return "other";
}

/**
 * Get display name for provider
 */
export function getProviderDisplayName(provider: string): string {
  const names: Record<string, string> = {
    openai: "OpenAI",
    anthropic: "Anthropic",
    google: "Google",
    "x.ai": "xAI",
    meta: "Meta",
    mistral: "Mistral AI",
    alibaba: "Alibaba",
    deepseek: "DeepSeek",
    other: "Other",
  };
  return names[provider] || provider;
}
