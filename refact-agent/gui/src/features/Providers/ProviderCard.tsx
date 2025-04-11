import React, { MouseEventHandler, useCallback } from "react";
import type {
  ConfiguredProvidersResponse,
  Provider,
} from "../../services/refact";
import { Card, Flex, Heading } from "@radix-ui/themes";
import { OnOffSwitch } from "../../components/OnOffSwitch/OnOffSwitch";

import styles from "./ProviderCard.module.css";
import { getProviderName } from "./getProviderName";

export type ProviderCardProps =
  | {
      provider: Provider;
      isSimplifiedProvider: false;
      setCurrentProvider: (
        provider: ConfiguredProvidersResponse["providers"][number],
      ) => void;
    }
  | {
      provider: ConfiguredProvidersResponse["providers"][number];
      isSimplifiedProvider: true;
      setCurrentProvider: (
        provider: ConfiguredProvidersResponse["providers"][number],
      ) => void;
    };

export const ProviderCard: React.FC<ProviderCardProps> = ({
  provider,
  isSimplifiedProvider,
  setCurrentProvider,
}) => {
  const handleClickOnProvider = useCallback(() => {
    setCurrentProvider(provider);
  }, [setCurrentProvider, provider]);

  const handleSwitchClick: MouseEventHandler<HTMLDivElement> = (event) => {
    // if (isUpdatingEnabled) return;
    event.stopPropagation();
    // eslint-disable-next-line no-console
    console.warn("Not implemented yet");
  };

  if (!isSimplifiedProvider) return null;
  return (
    <Card
      size="2"
      onClick={handleClickOnProvider}
      className={styles.providerCard}
    >
      <Flex align="center" justify="between">
        <Heading as="h6" size="2">
          {getProviderName(provider)}
        </Heading>
        <Flex align="center" gap="2">
          <OnOffSwitch
            isEnabled={provider.enabled}
            isUnavailable={provider.readonly}
            handleClick={handleSwitchClick}
          />
        </Flex>
      </Flex>
    </Card>
  );
};
