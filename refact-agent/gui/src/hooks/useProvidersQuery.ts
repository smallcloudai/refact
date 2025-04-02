import { type Provider, providersApi } from "../services/refact";

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

export async function useUpdateProviderMutation({
  provider,
}: {
  provider: Provider;
}) {
  const [mutationTrigger] = providersApi.useUpdateProviderMutation();
  return await mutationTrigger(provider).unwrap();
}
