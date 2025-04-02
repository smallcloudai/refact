import { modelsApi } from "../services/refact";

import type {
  DeleteModelRequestBody,
  GetModelArgs,
  UpdateModelRequestBody,
} from "../services/refact";

export function useGetModelsByProviderNameQuery({
  providerName,
}: {
  providerName: string;
}) {
  return modelsApi.useGetModelsQuery({ providerName });
}

export function useGetModelConfiguration(args: GetModelArgs) {
  return modelsApi.useGetModelQuery(args);
}

export async function useUpdateModelMutation(
  updateRequestBody: UpdateModelRequestBody,
) {
  const [mutationTrigger] = modelsApi.useUpdateModelMutation();
  return await mutationTrigger(updateRequestBody).unwrap();
}

export async function useDeleteModelMutation(
  deleteRequestBody: DeleteModelRequestBody,
) {
  const [mutationTrigger] = modelsApi.useDeleteModelMutation();
  return await mutationTrigger(deleteRequestBody).unwrap();
}
