import type { CapCost, CapsResponse } from "../services/refact";
import type { ModelCapabilities } from "../features/Providers/ProviderForm/ProviderModelsList/utils/groupModelsWithPricing";
import { extractProvider, getProviderDisplayName } from "./modelProviders";

export type EnrichedModel = {
  value: string;
  displayName: string;
  disabled: boolean;
  pricing?: CapCost;
  nCtx?: number;
  capabilities?: ModelCapabilities;
  isDefault?: boolean;
  isThinking?: boolean;
  isLight?: boolean;
  provider: string;
};

export type ModelGroup = {
  provider: string;
  displayName: string;
  models: EnrichedModel[];
};

type UsableModel = {
  value: string;
  textValue: string;
  disabled: boolean;
};

const PROVIDER_PRIORITY: Record<string, number> = {
  openai: 1,
  anthropic: 2,
  google: 3,
  "x.ai": 4,
  meta: 5,
  mistral: 6,
};

function extractCapabilities(
  capsModel: CapsResponse["chat_models"][string] | undefined,
): ModelCapabilities | undefined {
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (typeof capsModel !== "object" || !capsModel) return undefined;

  return {
    supportsTools: capsModel.supports_tools,
    supportsMultimodality: capsModel.supports_multimodality,
    supportsClicks: capsModel.supports_clicks,
    supportsAgent: capsModel.supports_agent,
    supportsReasoning: capsModel.supports_reasoning,
    supportsBoostReasoning: capsModel.supports_boost_reasoning,
  };
}

function getPricing(
  modelKey: string,
  displayName: string,
  caps: CapsResponse,
): CapCost | undefined {
  const pricing = caps.metadata?.pricing;
  if (!pricing) return undefined;

  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  return pricing[displayName] || pricing[modelKey.replace(/^refact\//, "")];
}

function getContextWindow(
  capsModel: CapsResponse["chat_models"][string] | undefined,
): number | undefined {
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (typeof capsModel !== "object" || !capsModel) return undefined;
  if ("n_ctx" in capsModel) return capsModel.n_ctx as number | undefined;
  return undefined;
}

export function enrichModels(
  usableModels: UsableModel[],
  caps: CapsResponse | undefined,
): EnrichedModel[] {
  if (!caps) {
    return usableModels.map((model) => ({
      value: model.value,
      displayName: model.textValue.replace(/^refact\//, ""),
      disabled: model.disabled,
      provider: extractProvider(model.value),
    }));
  }

  return usableModels.map((model) => {
    const modelKey = model.value;
    const displayName = model.textValue.replace(/^refact\//, "");
    const capsModel = caps.chat_models[modelKey];

    return {
      value: modelKey,
      displayName,
      disabled: model.disabled,
      pricing: getPricing(modelKey, displayName, caps),
      nCtx: getContextWindow(capsModel),
      capabilities: extractCapabilities(capsModel),
      isDefault: caps.chat_default_model === modelKey,
      isThinking: caps.chat_thinking_model === modelKey,
      isLight: caps.chat_light_model === modelKey,
      provider: extractProvider(modelKey),
    };
  });
}

function sortModelsInGroup(models: EnrichedModel[]): EnrichedModel[] {
  return [...models].sort((a, b) => {
    if (a.isDefault) return -1;
    if (b.isDefault) return 1;
    if (a.isThinking) return -1;
    if (b.isThinking) return 1;
    if (a.isLight) return -1;
    if (b.isLight) return 1;
    return a.displayName.localeCompare(b.displayName);
  });
}

export function groupModelsByProvider(models: EnrichedModel[]): ModelGroup[] {
  const groups = models.reduce<Record<string, EnrichedModel[]>>(
    (acc, model) => {
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
      if (!acc[model.provider]) {
        acc[model.provider] = [];
      }
      acc[model.provider].push(model);
      return acc;
    },
    {},
  );

  return Object.entries(groups)
    .map(([provider, providerModels]) => ({
      provider,
      displayName: getProviderDisplayName(provider),
      models: sortModelsInGroup(providerModels),
    }))
    .sort((a, b) => {
      const aPri = PROVIDER_PRIORITY[a.provider] || 999;
      const bPri = PROVIDER_PRIORITY[b.provider] || 999;
      if (aPri !== bPri) return aPri - bPri;
      return a.displayName.localeCompare(b.displayName);
    });
}

export function enrichAndGroupModels(
  usableModels: UsableModel[],
  caps: CapsResponse | undefined,
): ModelGroup[] {
  const enriched = enrichModels(usableModels, caps);
  return groupModelsByProvider(enriched);
}
