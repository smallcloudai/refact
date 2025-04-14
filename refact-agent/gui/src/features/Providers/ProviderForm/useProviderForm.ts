import isEqual from "lodash.isequal";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Provider } from "../../../services/refact";
import {
  useGetConfiguredProvidersQuery,
  useGetProviderQuery,
} from "../../../hooks/useProvidersQuery";

export function useProviderForm({ providerName }: { providerName: string }) {
  const { data: detailedProvider, isSuccess: isProviderLoadedSuccessfully } =
    useGetProviderQuery({
      providerName: providerName,
    });
  const { data: configuredProviders } = useGetConfiguredProvidersQuery();

  const [formValues, setFormValues] = useState<Provider | null>(null);
  const [areShowingExtraFields, setAreShowingExtraFields] = useState(false);

  useEffect(() => {
    if (detailedProvider) {
      setFormValues(detailedProvider);
    }
  }, [detailedProvider]);

  const shouldSaveButtonBeDisabled = useMemo(() => {
    if (!detailedProvider) return true;

    const isProviderConfigured = configuredProviders?.providers.some(
      (p) => p.name === providerName,
    );
    if (!isProviderConfigured) return false;

    return detailedProvider.readonly || isEqual(formValues, detailedProvider);
  }, [configuredProviders, detailedProvider, formValues, providerName]);

  const handleFormValuesChange = useCallback(
    (updatedProviderData: Provider) => {
      setFormValues(updatedProviderData);
    },
    [],
  );

  return {
    formValues,
    setFormValues,
    areShowingExtraFields,
    setAreShowingExtraFields,
    shouldSaveButtonBeDisabled,
    handleFormValuesChange,
    configuredProviders,
    detailedProvider,
    isProviderLoadedSuccessfully,
  };
}
