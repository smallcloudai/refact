import { RootState } from "../app/store";
import { getServerUrl } from "../services/refact/call_engine";

const TEMPLATE_KEYWORDS = ["cmdline", "mcp", "service"] as const;

export const formatIntegrationIconPath = (iconPath: string | undefined, state: RootState | null): string => {
  if (!state || !iconPath) {
    // Return a default icon path or placeholder
    return getServerUrl(state, '/v1/integration-icon/default.png');
  }

  if (TEMPLATE_KEYWORDS.some((keyword) => iconPath.includes(keyword))) {
    return getServerUrl(state, '/v1/integration-icon/cmdline_TEMPLATE.png');
  }

  // If the path already includes /v1/, use it as is
  if (iconPath.includes('/v1/')) {
    const normalizedPath = iconPath.startsWith('/') ? iconPath : `/${iconPath}`;
    return getServerUrl(state, normalizedPath);
  }

  // Otherwise, add /v1/ prefix and ensure no double slashes
  const normalizedPath = iconPath.startsWith('/') ? iconPath.substring(1) : iconPath;
  return getServerUrl(state, `/v1/${normalizedPath}`);
};