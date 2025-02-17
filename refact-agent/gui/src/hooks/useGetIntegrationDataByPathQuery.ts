import { integrationsApi } from "../services/refact/integrations";
import { useGetPing } from "./useGetPing";
import { addToCacheOnMiss } from "../features/Integrations/integrationsSlice";
import { useAppDispatch } from "./useAppDispatch";
import { useEffect } from "react";

export const useGetIntegrationDataByPathQuery = (integrationPath: string) => {
  const ping = useGetPing();
  const skip = !ping.data;
  const dispatch = useAppDispatch();

  const integration = integrationsApi.useGetIntegrationByPathQuery(
    integrationPath,
    {
      skip,
    },
  );

  useEffect(() => {
    if (integration.data) {
      dispatch(addToCacheOnMiss(integration.data));
    }
  }, [dispatch, integration.data]);

  // TBD: add other methods for checking values here or else where?

  return {
    integration,
  };
};
