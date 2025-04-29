import type { SimplifiedProvider } from "../../services/refact";
import { BEAUTIFUL_PROVIDER_NAMES } from "./constants";

export function getProviderName(provider: SimplifiedProvider | string): string {
  if (typeof provider === "string") return BEAUTIFUL_PROVIDER_NAMES[provider];
  const maybeName = provider.name;
  if (!maybeName) return "Unknown Provider"; // TODO: throw error or think through it more
  const beautyName = BEAUTIFUL_PROVIDER_NAMES[maybeName] as string | undefined;
  return beautyName ? beautyName : maybeName;
}
