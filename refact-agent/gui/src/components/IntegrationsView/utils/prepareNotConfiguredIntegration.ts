import {
  IntegrationWithIconRecord,
  IntegrationWithIconRecordAndAddress,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";

export function prepareNotConfiguredIntegration(
  integration: IntegrationWithIconRecordAndAddress,
  integrations?: IntegrationWithIconRecord[],
): NotConfiguredIntegrationWithIconRecord | null {
  const similarIntegrations = integrations?.filter(
    (integr) => integr.integr_name === integration.integr_name,
  );
  if (!similarIntegrations) return null;

  const uniqueConfigPaths = Array.from(
    new Set(similarIntegrations.map((integr) => integr.integr_config_path)),
  );
  const uniqueProjectPaths = Array.from(
    new Set(similarIntegrations.map((integr) => integr.project_path)),
  );

  uniqueProjectPaths.sort((a, _b) => (a === "" ? -1 : 1));
  uniqueConfigPaths.sort((a, _b) => (a.includes(".config") ? -1 : 1));

  const integrationToConfigure: NotConfiguredIntegrationWithIconRecord = {
    ...integration,
    commandName: integration.commandName ? integration.commandName : undefined,
    wasOpenedThroughChat: integration.shouldIntermediatePageShowUp,
    integr_config_path: uniqueConfigPaths,
    project_path: uniqueProjectPaths,
    integr_config_exists: false,
  };

  return integrationToConfigure;
}
