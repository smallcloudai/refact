const TEMPLATE_KEYWORDS = ["mcp", "cmdline", "service"] as const;

function getFileNameFromPath(path: string) {
  const parts = path.split("/");
  return parts[parts.length - 1];
}

function getIntegrationTypeFromFileName(fileName: string) {
  return fileName.split("_")[0];
}

export const formatIntegrationIconPath = (iconPath: string) => {
  if (TEMPLATE_KEYWORDS.some((keyword) => iconPath.includes(keyword))) {
    const fileName = getFileNameFromPath(iconPath);
    const integrationType = getIntegrationTypeFromFileName(fileName);

    return `/integration-icon/${integrationType}_TEMPLATE.png`;
  }
  return iconPath;
};
