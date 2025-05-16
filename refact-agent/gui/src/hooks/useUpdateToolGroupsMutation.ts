import { toolsApi } from "../services/refact";

export function useUpdateToolGroupsMutation() {
  const [mutationTrigger, mutationResult] =
    toolsApi.useUpdateToolGroupsMutation();

  return { mutationTrigger, mutationResult };
}
