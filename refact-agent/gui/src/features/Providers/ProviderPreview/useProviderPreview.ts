import { useCallback } from "react";

import { useAppDispatch } from "../../../hooks";
import { useUpdateProviderMutation } from "../../../hooks/useProvidersQuery";

import { setInformation } from "../../Errors/informationSlice";
import { providersApi } from "../../../services/refact";

import { getProviderName } from "../getProviderName";

import type { Provider, SimplifiedProvider } from "../../../services/refact";

export function useProviderPreview(
  handleSetCurrentProvider: (
    provider: SimplifiedProvider<"name" | "enabled" | "readonly"> | null,
  ) => void,
) {
  const dispatch = useAppDispatch();
  const updateProvider = useUpdateProviderMutation();

  const handleSaveChanges = useCallback(
    async (updatedProviderData: Provider) => {
      const response = await updateProvider(updatedProviderData);
      if (response.error) return;
      const actions = [
        setInformation(
          `Provider ${getProviderName(
            updatedProviderData,
          )} updated successfully`,
        ),
        providersApi.util.resetApiState(),
      ];
      actions.forEach((action) => dispatch(action));
      handleSetCurrentProvider(null);
    },
    [dispatch, handleSetCurrentProvider, updateProvider],
  );

  const handleDiscardChanges = useCallback(() => {
    handleSetCurrentProvider(null);
  }, [handleSetCurrentProvider]);

  return {
    updateProvider,
    handleDiscardChanges,
    handleSaveChanges,
  };
}
