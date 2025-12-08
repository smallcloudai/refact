import React, { useMemo } from "react";
import {
  Dialog,
  Flex,
  Text,
  Button,
  Badge,
  ScrollArea,
  Tooltip,
} from "@radix-ui/themes";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { useCapsForToolUse } from "../../hooks";
import { CapabilityIcons } from "../../features/Providers/ProviderForm/ProviderModelsList/components";
import type { ModelCapabilities } from "../../features/Providers/ProviderForm/ProviderModelsList/utils/groupModelsWithPricing";
import {
  formatContextWindow,
  formatPricing,
} from "../../features/Providers/ProviderForm/ProviderModelsList/utils/groupModelsWithPricing";
import type { CapCost } from "../../services/refact";
import {
  extractProvider,
  getProviderDisplayName,
} from "../../utils/modelProviders";

export type EnhancedModelSelectorProps = {
  disabled?: boolean;
};

type EnrichedModel = {
  name: string;
  displayName: string;
  value: string;
  disabled: boolean;
  pricing?: CapCost;
  nCtx?: number;
  capabilities?: ModelCapabilities;
  isDefault?: boolean;
  isThinking?: boolean;
  isLight?: boolean;
};



/**
 * Extract capabilities from chat model
 */
function extractCapabilitiesFromCaps(
  modelKey: string,
  caps: ReturnType<typeof useCapsForToolUse>["data"],
): ModelCapabilities | undefined {
  if (!caps) return undefined;

  const capsModel = caps.chat_models[modelKey];
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
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
 * Get pricing for a model from caps
 */
function getPricing(
  modelName: string,
  caps: ReturnType<typeof useCapsForToolUse>["data"],
): CapCost | undefined {
  if (!caps?.metadata?.pricing) return undefined;

  const pricing = caps.metadata.pricing;

  // Try exact match
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (pricing[modelName]) return pricing[modelName];

  // Try without "refact/" prefix
  const nameWithoutProvider = modelName.replace(/^refact\//, "");
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  if (pricing[nameWithoutProvider]) return pricing[nameWithoutProvider];

  return undefined;
}

export const EnhancedModelSelector: React.FC<EnhancedModelSelectorProps> = ({
  disabled,
}) => {
  const capsForToolUse = useCapsForToolUse();
  const [open, setOpen] = React.useState(false);

  const groupedModels = useMemo(() => {
    const caps = capsForToolUse.data;
    if (!caps) return [];

    const enrichedModels: EnrichedModel[] =
      capsForToolUse.usableModelsForPlan.map((model) => {
        const modelKey = model.value;
        const displayName = model.textValue.replace(/^refact\//, "");
        const capsModel = caps.chat_models[modelKey];
        const nCtx =
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          typeof capsModel === "object" && capsModel && "n_ctx" in capsModel
            ? (capsModel.n_ctx as number | undefined)
            : undefined;

        return {
          name: displayName,
          displayName,
          value: model.value,
          disabled: model.disabled,
          pricing: getPricing(displayName, caps),
          nCtx,
          capabilities: extractCapabilitiesFromCaps(modelKey, caps),
          isDefault: caps.chat_default_model === modelKey,
          isThinking: caps.chat_thinking_model === modelKey,
          isLight: caps.chat_light_model === modelKey,
        };
      });

    // Group by provider
    const groups = enrichedModels.reduce<Record<string, EnrichedModel[]>>(
      (acc, model) => {
        const provider = extractProvider(model.value);
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
        if (!acc[provider]) {
          acc[provider] = [];
        }
        acc[provider].push(model);
        return acc;
      },
      {},
    );

    // Convert to array and sort
    return Object.entries(groups)
      .map(([provider, models]) => ({
        provider,
        displayName: getProviderDisplayName(provider),
        models: models.sort((a, b) => {
          // Default first
          if (a.isDefault) return -1;
          if (b.isDefault) return 1;
          // Thinking second
          if (a.isThinking) return -1;
          if (b.isThinking) return 1;
          // Light third
          if (a.isLight) return -1;
          if (b.isLight) return 1;
          // Otherwise alphabetical
          return a.displayName.localeCompare(b.displayName);
        }),
      }))
      .sort((a, b) => {
        // Prioritize major providers
        const priority: Record<string, number> = {
          openai: 1,
          anthropic: 2,
          google: 3,
          "x.ai": 4,
          meta: 5,
          mistral: 6,
        };
        const aPri = priority[a.provider] || 999;
        const bPri = priority[b.provider] || 999;
        if (aPri !== bPri) return aPri - bPri;
        return a.displayName.localeCompare(b.displayName);
      });
  }, [capsForToolUse.usableModelsForPlan, capsForToolUse.data]);

  const currentModelName = capsForToolUse.currentModel.replace(/^refact\//, "");

  if (!capsForToolUse.data || groupedModels.length === 0) {
    return (
      <Text size="1" color="gray">
        model: {currentModelName}
      </Text>
    );
  }

  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      <Dialog.Trigger>
        <Button
          variant="ghost"
          size="1"
          disabled={disabled}
          style={{
            cursor: disabled ? "not-allowed" : "pointer",
            padding: "0 4px",
            minHeight: "20px",
            height: "20px",
            opacity: disabled ? 0.5 : 1,
          }}
        >
          <Text size="1" color="gray">
            model: {currentModelName}
          </Text>
        </Button>
      </Dialog.Trigger>

      <Dialog.Content
        style={{ maxWidth: 700, maxHeight: "80vh" }}
        aria-describedby={undefined}
      >
        <Dialog.Title>Select Model</Dialog.Title>

        <ScrollArea
          style={{ maxHeight: "calc(80vh - 120px)" }}
          scrollbars="vertical"
        >
          <Flex direction="column" gap="4" py="2">
            {groupedModels.map((group) => (
              <Flex key={group.provider} direction="column" gap="2">
                <Text size="2" weight="bold" color="gray">
                  {group.displayName}
                </Text>

                <Flex direction="column" gap="2">
                  {group.models.map((model) => (
                    <ModelCard
                      key={model.value}
                      model={model}
                      isSelected={model.value === capsForToolUse.currentModel}
                      onSelect={() => {
                        capsForToolUse.setCapModel(model.value);
                        setOpen(false);
                      }}
                    />
                  ))}
                </Flex>
              </Flex>
            ))}
          </Flex>
        </ScrollArea>

        <Flex gap="3" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">
              Close
            </Button>
          </Dialog.Close>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};

type ModelCardProps = {
  model: EnrichedModel;
  isSelected: boolean;
  onSelect: () => void;
};

const ModelCard: React.FC<ModelCardProps> = ({
  model,
  isSelected,
  onSelect,
}) => {
  return (
    <Flex
      direction="column"
      p="3"
      gap="2"
      style={{
        border: isSelected
          ? "2px solid var(--accent-9)"
          : "1px solid var(--gray-6)",
        borderRadius: "var(--radius-3)",
        cursor: model.disabled ? "not-allowed" : "pointer",
        backgroundColor: isSelected
          ? "var(--accent-2)"
          : "var(--color-panel-solid)",
        opacity: model.disabled ? 0.5 : 1,
        transition: "all 0.2s ease",
      }}
      onClick={model.disabled ? undefined : onSelect}
      onMouseEnter={(e) => {
        if (!model.disabled && !isSelected) {
          e.currentTarget.style.borderColor = "var(--gray-8)";
        }
      }}
      onMouseLeave={(e) => {
        if (!model.disabled && !isSelected) {
          e.currentTarget.style.borderColor = "var(--gray-6)";
        }
      }}
    >
      <Flex justify="between" align="start">
        <Flex direction="column" gap="1" flexGrow="1">
          <Flex align="center" gap="2" wrap="wrap">
            <Text size="2" weight="bold">
              {model.displayName}
            </Text>

            {model.isDefault && (
              <Badge size="1" color="blue">
                Default
              </Badge>
            )}
            {model.isThinking && (
              <Badge size="1" color="purple">
                Reasoning
              </Badge>
            )}
            {model.isLight && (
              <Badge size="1" color="green">
                Light
              </Badge>
            )}
          </Flex>

          <Flex align="center" gap="3" wrap="wrap">
            {model.pricing && (
              <Tooltip content="Price per 1M tokens (prompt/output)">
                <Flex align="center" gap="1">
                  <Text size="1" color="gray">
                    {formatPricing(model.pricing, true)}
                  </Text>
                  <InfoCircledIcon width={12} height={12} color="gray" />
                </Flex>
              </Tooltip>
            )}

            {model.nCtx && (
              <Tooltip content={`Context window: ${model.nCtx.toLocaleString()} tokens`}>
                <Flex align="center" gap="1">
                  <Text size="1" color="gray">
                    {formatContextWindow(model.nCtx)}
                  </Text>
                  <InfoCircledIcon width={12} height={12} color="gray" />
                </Flex>
              </Tooltip>
            )}

            {model.capabilities && (
              <CapabilityIcons capabilities={model.capabilities} />
            )}
          </Flex>
        </Flex>

        {isSelected && (
          <Badge size="1" color="blue">
            Selected
          </Badge>
        )}
      </Flex>
    </Flex>
  );
};
