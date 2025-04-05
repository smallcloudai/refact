import React, { useCallback, useEffect, useState } from "react";

import { Button, Flex, Heading, Select, Text } from "@radix-ui/themes";
import { ProviderCard } from "../ProviderCard";

import type {
  ConfiguredProvidersResponse,
  SimplifiedProvider,
} from "../../../services/refact";
import { useGetProviderTemplatesQuery } from "../../../hooks/useProvidersQuery";
import { BEAUTIFUL_PROVIDER_NAMES } from "../constants";

export type ConfiguredProvidersViewProps = {
  configuredProviders: ConfiguredProvidersResponse["providers"];
  handleSetCurrentProvider: (
    provider: ConfiguredProvidersResponse["providers"][number],
  ) => void;
};

export const ConfiguredProvidersView: React.FC<
  ConfiguredProvidersViewProps
> = ({ configuredProviders, handleSetCurrentProvider }) => {
  const [potentialCurrentProvider, setPotentialCurrentProvider] =
    useState<SimplifiedProvider<"name">>();

  const { data: providerTemplatesData } = useGetProviderTemplatesQuery();

  const handlePotentialCurrentProvider = useCallback((value: string) => {
    setPotentialCurrentProvider({
      name: value,
    });
  }, []);

  const handleAddNewProvider = useCallback(() => {
    if (!potentialCurrentProvider) return;

    handleSetCurrentProvider({
      name: potentialCurrentProvider.name,
      enabled: true,
      readonly: false,
    });
  }, [handleSetCurrentProvider, potentialCurrentProvider]);

  useEffect(() => {
    if (providerTemplatesData) {
      setPotentialCurrentProvider(providerTemplatesData.provider_templates[0]);
    }
  }, [providerTemplatesData]);

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
        {configuredProviders.map((provider, idx) => (
          <ProviderCard
            key={`${provider.name}_${idx}`}
            provider={provider}
            isSimplifiedProvider
            setCurrentProvider={handleSetCurrentProvider}
          />
        ))}
      </Flex>
      {providerTemplatesData && (
        <Flex direction="column" gap="2">
          <Heading as="h3" size="3">
            Add new provider
          </Heading>
          <Select.Root
            defaultValue={providerTemplatesData.provider_templates[0].name}
            size="2"
            onValueChange={handlePotentialCurrentProvider}
          >
            <Select.Trigger />
            <Select.Content variant="solid" position="popper">
              {providerTemplatesData.provider_templates.map((provider) => {
                return (
                  <Select.Item key={provider.name} value={provider.name}>
                    {BEAUTIFUL_PROVIDER_NAMES[provider.name]}
                  </Select.Item>
                );
              })}
            </Select.Content>
          </Select.Root>
          {potentialCurrentProvider && (
            <Button variant="outline" onClick={handleAddNewProvider}>
              Configure{" "}
              {BEAUTIFUL_PROVIDER_NAMES[potentialCurrentProvider.name]}
            </Button>
          )}
        </Flex>
      )}
    </Flex>
  );
};
