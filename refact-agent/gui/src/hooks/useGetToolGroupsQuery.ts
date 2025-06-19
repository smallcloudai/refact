import { toolsApi } from "../services/refact/tools";
import { useHasCaps } from "./useHasCaps";

// here
export const useGetToolGroupsQuery = () => {
  const hasCaps = useHasCaps();
  return toolsApi.useGetToolGroupsQuery(undefined, { skip: !hasCaps });
};

export const useGetToolsLazyQuery = () => {
  return toolsApi.useLazyGetToolGroupsQuery();
};

export const useCheckForConfirmationMutation = () => {
  return toolsApi.useCheckForConfirmationMutation();
};
