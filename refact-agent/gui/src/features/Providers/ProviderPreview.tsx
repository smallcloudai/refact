import React, { useCallback } from "react";
import {
  Provider,
  providersApi,
  SimplifiedProvider,
} from "../../services/refact";
import { Flex } from "@radix-ui/themes";
import { ProviderForm } from "./ProviderForm";
import { useUpdateProviderMutation } from "../../hooks/useProvidersQuery";
import { useAppDispatch } from "../../hooks";
import { setInformation } from "../Errors/informationSlice";
import { BEAUTIFUL_PROVIDER_NAMES } from "./constants";

export type ProviderPreviewProps = {
  configuredProviders: SimplifiedProvider<"name" | "enabled" | "readonly">[];
  currentProvider: SimplifiedProvider<"name" | "enabled" | "readonly">;
  handleSetCurrentProvider: (
    provider: SimplifiedProvider<"name" | "enabled" | "readonly"> | null,
  ) => void;
};

export const ProviderPreview: React.FC<ProviderPreviewProps> = ({
  currentProvider,
  handleSetCurrentProvider,
}) => {
  const dispatch = useAppDispatch();
  const updateProvider = useUpdateProviderMutation();

  const handleSaveChanges = useCallback(
    async (updatedProviderData: Provider) => {
      const response = await updateProvider(updatedProviderData);
      if (response.error) return;
      const actions = [
        setInformation(
          `Provider ${
            BEAUTIFUL_PROVIDER_NAMES[updatedProviderData.name]
          } updated successfully`,
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

  return (
    <Flex direction="column" align="start">
      <ProviderForm
        currentProvider={currentProvider}
        handleSaveChanges={(updatedProviderData) =>
          void handleSaveChanges(updatedProviderData)
        }
        handleDiscardChanges={handleDiscardChanges}
      />
    </Flex>
  );
};
