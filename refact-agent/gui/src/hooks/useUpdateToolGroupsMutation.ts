import { toolsApi } from "../services/refact/tools";

export function useUpdateToolGroupsMutation() {
  const [mutationTrigger, mutationResult] =
    toolsApi.useUpdateToolGroupsMutation();

  return { mutationTrigger, mutationResult };
}
