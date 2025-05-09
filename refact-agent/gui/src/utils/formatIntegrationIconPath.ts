const TEMPLATE_KEYWORDS = ["cmdline", "service"] as const;

export const formatIntegrationIconPath = (iconPath: string) => {
  if (TEMPLATE_KEYWORDS.some((keyword) => iconPath.includes(keyword))) {
    return "/integration-icon/cmdline_TEMPLATE.png";
  }
  return iconPath;
};
