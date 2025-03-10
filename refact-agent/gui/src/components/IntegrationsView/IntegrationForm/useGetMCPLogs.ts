import { integrationsApi } from "../../../services/refact";

export function useGetMCPLogs(integrationPath: string) {
  return integrationsApi.useGetMCPLogsByPathQuery(integrationPath, {
    pollingInterval: 2000,
  });
}
