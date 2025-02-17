import { integrationsApi } from "../services/refact/integrations";
import { useGetPing } from "./useGetPing";

export const useGetIntegrationsQuery = () => {
  const ping = useGetPing();
  const skip = !ping.data;
  const integrations = integrationsApi.useGetAllIntegrationsQuery(undefined, {
    skip,
  });

  // const icons = integrationsApi.useGetIntegrationIconsQuery(undefined, {
  //   skip,
  // });

  return {
    integrations,
    // icons
  };
};
