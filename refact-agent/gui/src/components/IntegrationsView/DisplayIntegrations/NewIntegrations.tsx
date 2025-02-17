import { Flex, Grid, Heading } from "@radix-ui/themes";
import { FC } from "react";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { IntegrationCard } from "./IntegrationCard";

type NewIntegrationsProps = {
  availableIntegrationsToConfigure?: NotConfiguredIntegrationWithIconRecord[];
  handleIntegrationShowUp: (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => void;
};

export const NewIntegrations: FC<NewIntegrationsProps> = ({
  availableIntegrationsToConfigure,
  handleIntegrationShowUp,
}) => (
  <Flex direction="column" gap="4" align="start">
    <Heading as="h4" size="3">
      <Flex align="start" gap="3" justify="center">
        Add new integration
      </Flex>
    </Heading>
    <Grid
      align="stretch"
      gap="3"
      columns={{ initial: "2", xs: "3", sm: "4", md: "5" }}
      width="100%"
    >
      {availableIntegrationsToConfigure &&
        Object.entries(availableIntegrationsToConfigure).map(
          ([_projectPath, integration], index) => (
            <IntegrationCard
              isNotConfigured
              key={`project-${index}-${JSON.stringify(
                integration.integr_config_path,
              )}`}
              integration={integration}
              handleIntegrationShowUp={handleIntegrationShowUp}
            />
          ),
        )}
    </Grid>
  </Flex>
);
