import { toolsApi } from "../services/refact/tools";
import { useHasCaps } from "./useHasCaps";

export const useGetToolsQuery = () => {
  const hasCaps = useHasCaps();
  return toolsApi.useGetToolsQuery(undefined, { skip: !hasCaps });
};

export const useGetToolsLazyQuery = () => {
  return toolsApi.useLazyGetToolsQuery();
};

export const useCheckForConfirmationMutation = () => {
  return toolsApi.useCheckForConfirmationMutation();
};
