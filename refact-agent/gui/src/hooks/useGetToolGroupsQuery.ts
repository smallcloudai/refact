import { toolsApi } from "../services/refact/tools";
import { useGetPing } from "./useGetPing";
// import { useHasCaps } from "./useHasCaps";

// can remove
export const useGetToolGroupsQuery = () => {
  const ping = useGetPing();
  return toolsApi.useGetToolGroupsQuery(undefined, { skip: !ping.data });
};

// use this
export const useGetToolsLazyQuery = () => {
  return toolsApi.useLazyGetToolGroupsQuery();
};
