import { useCallback, useEffect, useMemo, useState } from "react";
import type { SimplifiedProvider } from "../../../services/refact";
import { useGetProviderTemplatesQuery } from "../../../hooks/useProvidersQuery";
import { ConfiguredProvidersViewProps } from "./ConfiguredProvidersView";

export function useGetConfiguredProvidersView({
  configuredProviders,
  handleSetCurrentProvider,
}: {
  configuredProviders: ConfiguredProvidersViewProps["configuredProviders"];
  handleSetCurrentProvider: ConfiguredProvidersViewProps["handleSetCurrentProvider"];
}) {
  const { data: providerTemplatesData } = useGetProviderTemplatesQuery();

  const notConfiguredProviderTemplates = useMemo(() => {
    return providerTemplatesData
      ? providerTemplatesData.provider_templates.reduce<
          SimplifiedProvider<"name">[]
        >((acc, provider) => {
          if (!configuredProviders.some((p) => p.name === provider.name))
            acc.push(provider);
          return acc;
        }, [])
      : [];
  }, [configuredProviders, providerTemplatesData]);

  const [potentialCurrentProvider, setPotentialCurrentProvider] = useState<
    SimplifiedProvider<"name"> | undefined
  >(notConfiguredProviderTemplates[0] || undefined);

  const sortedConfiguredProviders = useMemo(() => {
    return [...configuredProviders].sort((a, b) => {
      const getPriority = (provider: { name: string }) => {
        if (
          provider.name === "refact" ||
          provider.name === "refact_self_hosted"
        )
          return 0;
        if (provider.name === "custom") return 2;
        return 1;
      };

      const priorityA = getPriority(a);
      const priorityB = getPriority(b);

      if (priorityA !== priorityB) {
        return priorityA - priorityB;
      }

      return a.name.localeCompare(b.name);
    });
  }, [configuredProviders]);

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
      supports_completion: false,
    });
  }, [handleSetCurrentProvider, potentialCurrentProvider]);

  useEffect(() => {
    if (notConfiguredProviderTemplates.length > 0) {
      setPotentialCurrentProvider(notConfiguredProviderTemplates[0]);
    }
  }, [notConfiguredProviderTemplates]);

  return {
    handlePotentialCurrentProvider,
    handleAddNewProvider,
    sortedConfiguredProviders,
    notConfiguredProviderTemplates,
    potentialCurrentProvider,
  };
}
