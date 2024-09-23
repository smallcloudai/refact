import { toolsApi } from "../services/refact/tools";

export const useGetToolsQuery = () => {
  return toolsApi.useGetToolsQuery(undefined);
};
