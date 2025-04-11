import React, { useCallback } from "react";
import {
  Provider,
  providersApi,
  SimplifiedProvider,
} from "../../services/refact";
import { Flex, Heading } from "@radix-ui/themes";
import { ProviderForm } from "./ProviderForm";
import { useUpdateProviderMutation } from "../../hooks/useProvidersQuery";
import { useAppDispatch } from "../../hooks";
import { setInformation } from "../Errors/informationSlice";
import { getProviderName } from "./getProviderName";

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

  return (
    <Flex direction="column" align="start" height="100%">
      <Heading as="h2" size="3" mb="4">
        {getProviderName(currentProvider)} Configuration
      </Heading>
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
