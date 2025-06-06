import { Flex } from "@radix-ui/themes";

import styles from "./IntegrationForm.module.css";
import { FC } from "react";
import {
  areAllFieldsBoolean,
  Integration,
  IntegrationFieldValue,
} from "../../../services/refact";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { DeletePopover } from "../../DeletePopover";

type FormAvailabilityAndDeleteProps = {
  integration: Integration;
  isApplying: boolean;
  isDeletingIntegration: boolean;
  onDelete: (path: string) => void;
  onChange: (fieldKey: string, fieldValue: IntegrationFieldValue) => void;
  formValues: Record<string, IntegrationFieldValue> | null;
};

export const FormAvailabilityAndDelete: FC<FormAvailabilityAndDeleteProps> = ({
  integration,
  onDelete,
  onChange,
  isApplying,
  isDeletingIntegration,
  formValues,
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
        {formValues?.available &&
          areAllFieldsBoolean(formValues.available) &&
          Object.entries(formValues.available).map(([key, value]) => (
            <IntegrationAvailability
              key={key}
              fieldName={key}
              value={value}
              onChange={() =>
                onChange("available", {
                  ...(formValues.available as Record<string, boolean>),
                  [key]: !value,
                })
              }
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
