import React from "react";
import { Flex, Text, Badge } from "@radix-ui/themes";
import { CapabilityIcons } from "../../features/Providers/ProviderForm/ProviderModelsList/components";
import type { ModelCapabilities } from "../../features/Providers/ProviderForm/ProviderModelsList/utils/groupModelsWithPricing";
import {
  formatContextWindow,
  formatPricing,
} from "../../features/Providers/ProviderForm/ProviderModelsList/utils/groupModelsWithPricing";
import type { CapCost } from "../../services/refact";

export type RichModelData = {
  displayName: string;
  pricing?: CapCost;
  nCtx?: number;
  capabilities?: ModelCapabilities;
  isDefault?: boolean;
  isThinking?: boolean;
  isLight?: boolean;
};

type RichModelSelectItemProps = RichModelData;

export const RichModelSelectItem: React.FC<RichModelSelectItemProps> = ({
  displayName,
  pricing,
  nCtx,
  capabilities,
  isDefault,
  isThinking,
  isLight,
}) => {
  return (
    <Flex direction="column" gap="0" style={{ lineHeight: 1.3 }}>
      <Flex align="center" gap="2">
        <Text size="2" weight="medium" style={{ lineHeight: 1.4 }}>
          {displayName}
        </Text>
        {isDefault && (
          <Badge size="1" color="blue" variant="soft">
            Default
          </Badge>
        )}
        {isThinking && (
          <Badge size="1" color="purple" variant="soft">
            Reasoning
          </Badge>
        )}
        {isLight && (
          <Badge size="1" color="green" variant="soft">
            Light
          </Badge>
        )}
      </Flex>

      <Flex align="center" gap="2" style={{ opacity: 0.6, marginTop: 2 }}>
        {pricing && (
          <Text
            size="1"
            color="gray"
            title={formatPricing(pricing, false)}
            style={{ cursor: "help", fontSize: "11px" }}
          >
            {formatPricing(pricing, true)}
          </Text>
        )}
        {nCtx && (
          <Text
            size="1"
            color="gray"
            title={`Context window: ${nCtx.toLocaleString()} tokens`}
            style={{ cursor: "help", fontSize: "11px" }}
          >
            {formatContextWindow(nCtx)}
          </Text>
        )}
        {capabilities && <CapabilityIcons capabilities={capabilities} />}
      </Flex>
    </Flex>
  );
};
