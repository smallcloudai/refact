import { Flex, Text } from "@radix-ui/themes";

import { FC } from "react";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { GlobalIntegrations } from "./GlobalIntegrations";
import { NewIntegrations } from "./NewIntegrations";
import { ProjectIntegrations } from "./ProjectIntegrations";

type IntegrationsListProps = {
  globalIntegrations?: IntegrationWithIconRecord[];
  groupedProjectIntegrations?: Record<string, IntegrationWithIconRecord[]>;
  availableIntegrationsToConfigure?: NotConfiguredIntegrationWithIconRecord[];
  handleIntegrationShowUp: (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => void;
};

export const IntegrationsList: FC<IntegrationsListProps> = ({
  globalIntegrations,
  groupedProjectIntegrations,
  availableIntegrationsToConfigure,
  handleIntegrationShowUp,
}) => {
  return (
    <Flex direction="column" width="100%" gap="4">
      <Text my="2">
        Integrations allow Refact.ai Agent to interact with other services and
        tools
      </Text>
      <GlobalIntegrations
        globalIntegrations={globalIntegrations}
        handleIntegrationShowUp={handleIntegrationShowUp}
      />
      <ProjectIntegrations
        groupedProjectIntegrations={groupedProjectIntegrations}
        handleIntegrationShowUp={handleIntegrationShowUp}
      />
      <NewIntegrations
        availableIntegrationsToConfigure={availableIntegrationsToConfigure}
        handleIntegrationShowUp={handleIntegrationShowUp}
      />
    </Flex>
  );
};
