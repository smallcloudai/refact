import React from "react";
import { Flex, Heading } from "@radix-ui/themes";

import { ProviderForm } from "../ProviderForm";

import { useProviderPreview } from "./useProviderPreview";
import { getProviderName } from "../getProviderName";

import type { SimplifiedProvider } from "../../../services/refact";

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
  const { handleDiscardChanges, handleSaveChanges } = useProviderPreview(
    handleSetCurrentProvider,
  );

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
