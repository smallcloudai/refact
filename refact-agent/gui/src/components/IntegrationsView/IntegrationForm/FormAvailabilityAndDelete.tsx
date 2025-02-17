import { Flex } from "@radix-ui/themes";

import styles from "./IntegrationForm.module.css";
import { FC } from "react";
import { Integration } from "../../../services/refact";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { IntegrationDeletePopover } from "../IntegrationDeletePopover";

type FormAvailabilityAndDeleteProps = {
  integration: Integration;
  availabilityValues: Record<string, boolean>;
  isApplying: boolean;
  isDeletingIntegration: boolean;
  handleAvailabilityChange: (fieldName: string, value: boolean) => void;
  onDelete: (path: string, name: string) => void;
};

export const FormAvailabilityAndDelete: FC<FormAvailabilityAndDeleteProps> = ({
  integration,
  availabilityValues,
  handleAvailabilityChange,
  onDelete,
  isApplying,
  isDeletingIntegration,
}) => {
  const { integr_values, integr_config_path, integr_name } = integration;
  if (!integr_values) return null;
  return (
    <Flex align="start" justify="between">
      <Flex
        gap="4"
        mb="4"
        align="center"
        justify="between"
        className={styles.switchInline}
      >
        {integr_values.available &&
          Object.keys(integr_values.available).map((key) => (
            <IntegrationAvailability
              key={key}
              fieldName={key}
              value={availabilityValues[key]}
              onChange={handleAvailabilityChange}
            />
          ))}
      </Flex>
      <IntegrationDeletePopover
        integrationName={integr_name}
        integrationConfigPath={integr_config_path}
        isApplying={isApplying}
        isDeletingIntegration={isDeletingIntegration}
        handleDeleteIntegration={onDelete}
      />
    </Flex>
  );
};
