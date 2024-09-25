import { toolsApi } from "../services/refact/tools";
import { useHasCaps } from "./useHasCaps";

export const useGetToolsQuery = () => {
  const hasCaps = useHasCaps();
  return toolsApi.useGetToolsQuery(undefined, { skip: !hasCaps });
};
