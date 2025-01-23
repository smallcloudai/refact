import { integrationsApi } from "../services/refact/integrations";

export const useDeleteIntegrationByPath = () => {
  const [deleteIntegrationTrigger] =
    integrationsApi.useLazyDeleteIntegrationQuery();

  return { deleteIntegrationTrigger };
};
