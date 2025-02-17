import { Flex, Heading, Text } from "@radix-ui/themes";
import { FC } from "react";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { formatPathName } from "../../../utils/formatPathName";
import { Markdown } from "../../Markdown";
import { IntegrationCard } from "./IntegrationCard";

type ProjectIntegrationsProps = {
  groupedProjectIntegrations?: Record<string, IntegrationWithIconRecord[]>;
  handleIntegrationShowUp: (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => void;
};

export const ProjectIntegrations: FC<ProjectIntegrationsProps> = ({
  groupedProjectIntegrations,
  handleIntegrationShowUp,
}) => {
  if (!groupedProjectIntegrations) return null;

  return Object.entries(groupedProjectIntegrations).map(
    ([projectPath, integrations], index) => {
      const formattedProjectName = formatPathName(
        projectPath,
        "```.../",
        "/```",
      );

      return (
        <Flex
          key={`project-group-${index}`}
          direction="column"
          gap="4"
          align="start"
        >
          <Heading as="h4" size="3">
            <Flex align="start" gapX="3" gapY="1" justify="start" wrap="wrap">
              ⚙️ In
              <Markdown>{formattedProjectName}</Markdown>
              configured {integrations.length}{" "}
              {integrations.length !== 1 ? "integrations" : "integration"}
            </Flex>
          </Heading>
          <Text size="2" color="gray">
            Folder-specific integrations are local integrations, which are
            shared only in folder-specific scope.
          </Text>
          <Flex direction="column" align="start" gap="2" width="100%">
            {integrations.map((integration, subIndex) => (
              <IntegrationCard
                key={`project-${index}-${subIndex}-${integration.integr_config_path}`}
                integration={integration}
                handleIntegrationShowUp={handleIntegrationShowUp}
              />
            ))}
          </Flex>
        </Flex>
      );
    },
  );
};
