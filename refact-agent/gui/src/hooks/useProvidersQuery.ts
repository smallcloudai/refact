import { providersApi } from "../services/refact/providers";

export function useGetConfiguredProvidersQuery() {
  return providersApi.useGetConfiguredProvidersQuery(undefined);
}

export function useGetProviderTemplatesQuery() {
  return providersApi.useGetProviderTemplatesQuery(undefined);
}

export function useGetProviderQuery({
  providerName,
}: {
  providerName: string;
}) {
  return providersApi.useGetProviderQuery({ providerName });
}

export function useUpdateProviderMutation() {
  const [mutationTrigger] = providersApi.useUpdateProviderMutation();
  return mutationTrigger;
}

export function useDeleteProviderMutation() {
  const [mutationTrigger] = providersApi.useDeleteProviderMutation();
  return mutationTrigger;
}
