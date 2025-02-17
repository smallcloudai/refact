import { Flex, Switch } from "@radix-ui/themes";
import type { FC } from "react";
import { CustomLabel } from "../CustomFieldsAndWidgets";
import { toPascalCase } from "../../../utils/toPascalCase";

type IntegrationAvailabilityProps = {
  fieldName: string;
  value: boolean;
  onChange: (fieldName: string, value: boolean) => void;
};

export const IntegrationAvailability: FC<IntegrationAvailabilityProps> = ({
  fieldName,
  value,
  onChange,
}) => {
  const handleSwitchChange = (checked: boolean) => {
    onChange(fieldName, checked);
  };

  // TODO: temporal solution to hide the switch for isolated mode
  if (fieldName === "when_isolated") return null;

  const handleLabelClick = () => {
    handleSwitchChange(!value);
  };

  return (
    <Flex style={{ marginBottom: "0.75rem" }}>
      <Flex align="center" justify="between" gap="3">
        <Switch
          id={`switch-${fieldName}`}
          size="2"
          checked={value}
          onCheckedChange={handleSwitchChange}
        />
        <label htmlFor={`switch-${fieldName}`} onClick={handleLabelClick}>
          <CustomLabel
            label={toPascalCase(
              fieldName === "on_your_laptop" ? "enable" : "run_in_docker",
            )}
          />
        </label>
      </Flex>
    </Flex>
  );
};
