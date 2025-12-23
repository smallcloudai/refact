import { modelsApi } from "../services/refact";

import type { GetModelArgs, GetModelDefaultsArgs } from "../services/refact";

export function useGetModelsByProviderNameQuery({
  providerName,
}: {
  providerName: string;
}) {
  return modelsApi.useGetModelsQuery({ providerName });
}

export function useGetModelConfiguration(args: GetModelArgs) {
  return modelsApi.useGetModelQuery(args, { skip: !args.modelName });
}

export function useGetModelDefaults(args: GetModelDefaultsArgs) {
  return modelsApi.useGetModelDefaultsQuery(args, { skip: !args.providerName });
}

export function useGetCompletionModelFamiliesQuery() {
  return modelsApi.useGetCompletionModelFamiliesQuery(undefined);
}

export function useGetLazyModelConfiguration() {
  const [mutationTrigger] = modelsApi.useLazyGetModelQuery();
  return mutationTrigger;
}

export function useUpdateModelMutation() {
  const [mutationTrigger] = modelsApi.useUpdateModelMutation();
  return mutationTrigger;
}

export function useDeleteModelMutation() {
  const [mutationTrigger] = modelsApi.useDeleteModelMutation();
  return mutationTrigger;
}
