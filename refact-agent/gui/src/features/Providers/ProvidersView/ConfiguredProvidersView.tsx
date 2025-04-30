import React from "react";

import { Button, Flex, Heading, Select, Text } from "@radix-ui/themes";
import { ProviderCard } from "../ProviderCard/ProviderCard";

import type { ConfiguredProvidersResponse } from "../../../services/refact";
import { getProviderName } from "../getProviderName";
import { useGetConfiguredProvidersView } from "./useConfiguredProvidersView";

export type ConfiguredProvidersViewProps = {
  configuredProviders: ConfiguredProvidersResponse["providers"];
  handleSetCurrentProvider: (
    provider: ConfiguredProvidersResponse["providers"][number],
  ) => void;
};

export const ConfiguredProvidersView: React.FC<
  ConfiguredProvidersViewProps
> = ({ configuredProviders, handleSetCurrentProvider }) => {
  const {
    handleAddNewProvider,
    handlePotentialCurrentProvider,
    notConfiguredProviderTemplates,
    sortedConfiguredProviders,
    potentialCurrentProvider,
  } = useGetConfiguredProvidersView({
    configuredProviders,
    handleSetCurrentProvider,
  });

  return (
    <Flex direction="column" gap="2" justify="between" height="100%">
      <Flex direction="column" gap="2">
        <Flex direction="column" gap="1">
          <Heading as="h2" size="3">
            Configured Providers
          </Heading>
          <Text as="p" size="2" color="gray">
            Here you can navigate through the list of configured and available
            providers
          </Text>
        </Flex>
        {sortedConfiguredProviders.map((provider, idx) => (
          <ProviderCard
            key={`${provider.name}_${idx}`}
            provider={provider}
            setCurrentProvider={handleSetCurrentProvider}
          />
        ))}
      </Flex>
      {notConfiguredProviderTemplates.length > 0 && (
        <Flex direction="column" gap="2">
          <Heading as="h3" size="3">
            Add new provider
          </Heading>
          <Select.Root
            defaultValue={notConfiguredProviderTemplates[0].name}
            size="2"
            onValueChange={handlePotentialCurrentProvider}
          >
            <Select.Trigger />
            <Select.Content variant="solid" position="popper">
              {notConfiguredProviderTemplates.map((provider) => {
                return (
                  <Select.Item key={provider.name} value={provider.name}>
                    {getProviderName(provider)}
                  </Select.Item>
                );
              })}
            </Select.Content>
          </Select.Root>
          {potentialCurrentProvider && (
            <Button variant="outline" onClick={handleAddNewProvider}>
              Configure {getProviderName(potentialCurrentProvider)}
            </Button>
          )}
        </Flex>
      )}
    </Flex>
  );
};
