import React from "react";
import { Card, Flex, Heading } from "@radix-ui/themes";

import { OnOffSwitch } from "../../../components/OnOffSwitch/OnOffSwitch";
import { iconsMap } from "../icons/iconsMap";

import type { ConfiguredProvidersResponse } from "../../../services/refact";

import { getProviderName } from "../getProviderName";
import { useProviderCard } from "./useProviderCard";

import styles from "./ProviderCard.module.css";
import { useUpdateProvider } from "../useUpdateProvider";
import classNames from "classnames";

export type ProviderCardProps = {
  provider: ConfiguredProvidersResponse["providers"][number];
  setCurrentProvider: (
    provider: ConfiguredProvidersResponse["providers"][number],
  ) => void;
};

export const ProviderCard: React.FC<ProviderCardProps> = ({
  provider,
  setCurrentProvider,
}) => {
  const { isUpdatingEnabledState } = useUpdateProvider({
    provider,
  });

  const { handleClickOnProvider, handleSwitchClick } = useProviderCard({
    provider,
    setCurrentProvider,
  });

  return (
    <Card
      size="2"
      onClick={handleClickOnProvider}
      className={classNames(styles.providerCard, {
        [styles.providerCardDisabled]: isUpdatingEnabledState,
      })}
    >
      <Flex align="center" justify="between">
        <Flex gap="3" align="center">
          {iconsMap[provider.name]}
          <Heading as="h6" size="2">
            {getProviderName(provider)}
          </Heading>
        </Flex>
        {!provider.readonly && (
          <Flex align="center" gap="2">
            <OnOffSwitch
              isEnabled={provider.enabled}
              isUpdating={isUpdatingEnabledState}
              isUnavailable={isUpdatingEnabledState}
              handleClick={handleSwitchClick}
            />
          </Flex>
        )}
      </Flex>
    </Card>
  );
};
