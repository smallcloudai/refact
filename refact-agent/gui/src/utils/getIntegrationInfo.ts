import { toPascalCase } from "./toPascalCase";

export const getIntegrationInfo = (integrationName: string) => {
  const isMCP = integrationName.startsWith("mcp");
  const isCmdline = integrationName.startsWith("cmdline");
  const isService = integrationName.startsWith("service");

  const getDisplayName = () => {
    if (!integrationName.includes("TEMPLATE")) {
      return toPascalCase(integrationName);
    }
    if (isCmdline) return "Command-line Tool";
    if (isService) return "Command-line Service";
    if (isMCP) return "MCP Server";
    return "";
  };

  return {
    isMCP,
    isCmdline,
    isService,
    displayName: getDisplayName(),
  };
};
