import { toPascalCase } from "./toPascalCase";

export const getIntegrationInfo = (integrationName: string) => {
  const isMCPSse = integrationName.startsWith("mcp_sse");
  const isMCPStdio = !integrationName.startsWith("mcp_sse") && integrationName.includes("mcp");
  const isCmdline = integrationName.startsWith("cmdline");
  const isService = integrationName.startsWith("service");

  const getDisplayName = () => {
    if (!integrationName.includes("TEMPLATE")) {
      return toPascalCase(integrationName);
    }
    if (isCmdline) return "Command-line Tool";
    if (isService) return "Command-line Service";
    if (isMCPSse) return "MCP (Connect to SSE)";
    if (isMCPStdio) return "MCP (Run via stdio)";
    return "";
  };

  return {
    isMCP: isMCPSse || isMCPStdio,
    isCmdline,
    isService,
    displayName: getDisplayName(),
  };
};
