import { useCallback } from "react";

import { providersApi } from "../../services/refact";
import { useAppDispatch } from "../../hooks";

import { getProviderName } from "./getProviderName";
import { setError } from "../../features/Errors/errorsSlice";
import { useProviderUpdateContext } from "./ProviderUpdateContext";

import type { ProviderCardProps } from "./ProviderCard";

export const useUpdateProvider = ({
  provider,
}: {
  provider: ProviderCardProps["provider"];
}) => {
  const dispatch = useAppDispatch();
  const { updatingProviders, setProviderUpdating } = useProviderUpdateContext();

  const [getProviderData] = providersApi.useLazyGetProviderQuery();
  const [saveProviderData] = providersApi.useUpdateProviderMutation();

  // Use the provider name as the key to track state
  // then get updating state from context
  const providerKey = provider.name;
  const isUpdatingEnabledState = updatingProviders[providerKey] || false;

  const updateProviderEnabledState = useCallback(async () => {
    setProviderUpdating(providerKey, true);

    const { data: providerData } = await getProviderData({
      providerName: provider.name,
    });

    if (!providerData) {
      setProviderUpdating(providerKey, false);
      return;
    }

    const enabled = providerData.enabled;

    const response = await saveProviderData({
      ...providerData,
      enabled: !enabled,
    });

    if (response.error) {
      dispatch(
        setError(
          `Error occurred on updating ${getProviderName(
            provider,
          )} configuration. Check if your provider configuration is correct`,
        ),
      );
      setProviderUpdating(providerKey, false);
      return;
    }

    dispatch(
      providersApi.util.invalidateTags([
        { type: "CONFIGURED_PROVIDERS", id: "LIST" },
      ]),
    );
    setTimeout(() => {
      setProviderUpdating(providerKey, false);
    }, 500);
  }, [
    dispatch,
    getProviderData,
    saveProviderData,
    provider,
    providerKey,
    setProviderUpdating,
  ]);

  return {
    updateProviderEnabledState,
    isUpdatingEnabledState,
  };
};
