import type {
  SimplifiedModel,
  ModelType,
  CodeChatModel,
} from "../../../../../services/refact";
import type { CapsResponse, CapCost } from "../../../../../services/refact";

export type UiModel = SimplifiedModel & {
  modelType: ModelType;
  pricing?: CapCost;
  pricingLabel?: string;
  nCtx?: number;
  nCtxLabel?: string;
  isDefault?: boolean;
  isLight?: boolean;
  isThinking?: boolean;
  capabilities?: ModelCapabilities;
};

export type ModelCapabilities = {
  supportsTools?: boolean;
  supportsMultimodality?: boolean;
  supportsClicks?: boolean;
  supportsAgent?: boolean;
  supportsReasoning?: string | null;
  supportsBoostReasoning?: boolean;
};

export type ModelGroup = {
  id: string;
  title: string;
  description?: string;
  models: UiModel[];
};

/**
 * Format context window size to human-readable format
 */
export function formatContextWindow(nCtx: number): string {
  if (nCtx >= 1_000_000) {
    return `${(nCtx / 1_000_000).toFixed(1)}M`.replace(".0M", "M");
  }
  if (nCtx >= 1_000) {
    return `${Math.round(nCtx / 1_000)}K`;
  }
  return nCtx.toString();
}

/**
 * Format pricing to compact string (in coins, $1 = 1000 coins)
 */
export function formatPricing(cost: CapCost, compact = true): string {
  // Convert dollars to coins ($1 = 1000 coins)
  const toCoins = (n?: number) =>
    typeof n === "number" && Number.isFinite(n) ? Math.round(n * 1000) : null;

  const promptCoins = toCoins(cost.prompt);
  const generatedCoins = toCoins(cost.generated);

  const fmt = (coins: number | null) =>
    coins !== null ? coins.toString() : "–";

  if (compact) {
    // Compact format for card display: "1000/5000 ⓒ" (prompt/output in coins)
    return `${fmt(promptCoins)}/${fmt(generatedCoins)} ⓒ`;
  }

  // Detailed format for tooltip/popup
  const parts = [
    `prompt: ${fmt(promptCoins)} ⓒ`,
    `output: ${fmt(generatedCoins)} ⓒ`,
  ];

  const cacheReadCoins = toCoins(cost.cache_read);
  const cacheCreationCoins = toCoins(cost.cache_creation);

  if (cacheReadCoins !== null) {
    parts.push(`cache read: ${fmt(cacheReadCoins)} ⓒ`);
  }
  if (cacheCreationCoins !== null) {
    parts.push(`cache create: ${fmt(cacheCreationCoins)} ⓒ`);
  }

  return parts.join(" • ") + " per 1M tokens";
}

/**
 * Try to find the pricing key in caps.metadata.pricing that corresponds to a given model.
 * Based on actual /caps response, keys are bare model names (e.g., "gpt-4.1")
 */
function pickPricingKey(args: {
  caps: CapsResponse;
  modelName: string;
}): string | null {
  const { caps, modelName } = args;
  const pricing = caps.metadata?.pricing;
  if (!pricing) return null;

  // Try exact match first (most common case)
  if (Object.prototype.hasOwnProperty.call(pricing, modelName)) {
    return modelName;
  }

  // Try without "refact/" prefix if present
  const nameWithoutProvider = modelName.replace(/^refact\//, "");
  if (Object.prototype.hasOwnProperty.call(pricing, nameWithoutProvider)) {
    return nameWithoutProvider;
  }

  return null;
}

/**
 * Extract capabilities from chat model
 */
function extractCapabilities(
  capsModel: CodeChatModel | undefined,
): ModelCapabilities | undefined {
  if (!capsModel || typeof capsModel !== "object") return undefined;

  return {
    supportsTools: capsModel.supports_tools,
    supportsMultimodality: capsModel.supports_multimodality,
    supportsClicks: capsModel.supports_clicks,
    supportsAgent: capsModel.supports_agent,
    supportsReasoning: capsModel.supports_reasoning,
    supportsBoostReasoning: capsModel.supports_boost_reasoning,
  };
}

/**
 * Attach pricing, context window & capability flags to each simplified model.
 * Works even if caps/metadata/pricing is missing.
 */
export function attachPricingAndCapabilities(
  models: SimplifiedModel[],
  { caps, modelType }: { caps?: CapsResponse; modelType: ModelType },
): UiModel[] {
  if (!caps) {
    // No caps → only attach modelType
    return models.map((m) => ({ ...m, modelType }));
  }

  const capsModels =
    modelType === "chat" ? caps.chat_models : caps.completion_models;

  return models.map((m) => {
    const capsModelKey = `refact/${m.name}`;
    const capsModel = capsModels[capsModelKey];

    const pricingKey = pickPricingKey({
      caps,
      modelName: m.name,
    });

    const pricing =
      pricingKey && caps.metadata?.pricing
        ? caps.metadata.pricing[pricingKey]
        : undefined;

    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    const nCtx = capsModel?.n_ctx;

    const uiModel: UiModel = {
      ...m,
      modelType,
      pricing,
      pricingLabel: pricing ? formatPricing(pricing) : undefined,
      nCtx,
      nCtxLabel: nCtx ? formatContextWindow(nCtx) : undefined,
    };

    // Chat-type specific flags
    if (modelType === "chat") {
      uiModel.isDefault = caps.chat_default_model === `refact/${m.name}`;
      uiModel.isLight = caps.chat_light_model === `refact/${m.name}`;
      uiModel.isThinking = caps.chat_thinking_model === `refact/${m.name}`;

      if (typeof capsModel === "object") {
        uiModel.capabilities = extractCapabilities(capsModel as CodeChatModel);
      }
    }

    // Completion-type default
    if (modelType === "completion") {
      uiModel.isDefault = caps.completion_default_model === `refact/${m.name}`;
    }

    return uiModel;
  });
}

/**
 * Group models for UI. Uses default / thinking / light groups when possible.
 * Falls back to a single group if there's no useful structure.
 */
export function groupModelsWithPricing(
  models: SimplifiedModel[],
  options: {
    caps?: CapsResponse;
    modelType: ModelType;
  },
): ModelGroup[] {
  const decorated = attachPricingAndCapabilities(models, options);

  // No caps at all → single group, preserves old UI semantics
  if (!options.caps) {
    return [
      {
        id: "all",
        title:
          options.modelType === "chat" ? "Chat models" : "Completion models",
        models: decorated,
      },
    ];
  }

  const defaultGroup: ModelGroup = {
    id: "default",
    title: "Default",
    description: "Used by default for this provider",
    models: [],
  };
  const thinkingGroup: ModelGroup = {
    id: "thinking",
    title: "Reasoning / Thinking",
    description: "Slower but better at complex reasoning",
    models: [],
  };
  const lightGroup: ModelGroup = {
    id: "light",
    title: "Light / Cheaper",
    description: "Cheaper / faster variants",
    models: [],
  };
  const otherGroup: ModelGroup = {
    id: "other",
    title: "Other models",
    models: [],
  };

  for (const m of decorated) {
    if (m.isDefault) {
      defaultGroup.models.push(m);
    } else if (m.isThinking) {
      thinkingGroup.models.push(m);
    } else if (m.isLight) {
      lightGroup.models.push(m);
    } else {
      otherGroup.models.push(m);
    }
  }

  const groups: ModelGroup[] = [];
  if (defaultGroup.models.length) groups.push(defaultGroup);
  if (thinkingGroup.models.length) groups.push(thinkingGroup);
  if (lightGroup.models.length) groups.push(lightGroup);
  if (otherGroup.models.length) groups.push(otherGroup);

  // If we didn't get any meaningful separation, collapse into a single group
  if (
    groups.length === 1 &&
    groups[0].id === "other" &&
    groups[0].models.length === decorated.length
  ) {
    return [
      {
        id: "all",
        title:
          options.modelType === "chat" ? "Chat models" : "Completion models",
        models: decorated,
      },
    ];
  }

  return groups;
}
