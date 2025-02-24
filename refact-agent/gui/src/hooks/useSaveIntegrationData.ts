import { Integration, integrationsApi } from "../services/refact/integrations";
import { removeFromCache } from "../features/Integrations";
import { useCallback } from "react";

export const useSaveIntegrationData = () => {
  const [mutationTrigger] = integrationsApi.useSaveIntegrationMutation();

  const saveIntegrationMutationTrigger = useCallback(
    (filePath: string, values: Integration["integr_values"]) => {
      const result = mutationTrigger({ filePath, values });
      removeFromCache(filePath);
      return result;
    },
    [mutationTrigger],
  );

  return { saveIntegrationMutationTrigger };
};
