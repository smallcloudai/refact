import { Flex, Heading, Text } from "@radix-ui/themes";
import { FC } from "react";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { IntegrationCard } from "./IntegrationCard";

type GlobalIntegrationsProps = {
  globalIntegrations?: IntegrationWithIconRecord[];
  handleIntegrationShowUp: (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => void;
};

export const GlobalIntegrations: FC<GlobalIntegrationsProps> = ({
  globalIntegrations,
  handleIntegrationShowUp,
}) => {
  return (
    <Flex
      align="start"
      direction="column"
      justify="between"
      gap="4"
      width="100%"
    >
      <Heading as="h4" size="3" style={{ width: "100%" }}>
        ⚙️ Globally configured {globalIntegrations?.length ?? 0}{" "}
        {(globalIntegrations?.length ?? 0) !== 1
          ? "integrations"
          : "integration"}
      </Heading>
      <Text size="2" color="gray">
        Global configurations are shared in your IDE and available for all your
        projects.
      </Text>
      {globalIntegrations && (
        <Flex direction="column" align="start" gap="3" width="100%">
          {globalIntegrations.map((integration, index) => (
            <IntegrationCard
              key={`${index}-${integration.integr_config_path}`}
              integration={integration}
              handleIntegrationShowUp={handleIntegrationShowUp}
            />
          ))}
        </Flex>
      )}
    </Flex>
  );
};
