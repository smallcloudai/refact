import { Flex } from "@radix-ui/themes";

import styles from "./IntegrationForm.module.css";
import { FC } from "react";
import { Integration } from "../../../services/refact";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { DeletePopover } from "../../DeletePopover";

type FormAvailabilityAndDeleteProps = {
  integration: Integration;
  availabilityValues: Record<string, boolean>;
  isApplying: boolean;
  isDeletingIntegration: boolean;
  handleAvailabilityChange: (fieldName: string, value: boolean) => void;
  onDelete: (path: string) => void;
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
      <DeletePopover
        itemName={integr_name}
        deleteBy={integr_config_path}
        isDisabled={isApplying}
        isDeleting={isDeletingIntegration}
        handleDelete={onDelete}
      />
    </Flex>
  );
};
