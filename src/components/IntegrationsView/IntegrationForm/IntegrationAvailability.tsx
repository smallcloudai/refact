import { DataList, Flex, Switch } from "@radix-ui/themes";
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

  return (
    <DataList.Item
      style={{
        marginBottom: "0.75rem",
      }}
    >
      <DataList.Label>
        <CustomLabel
          label={toPascalCase(
            fieldName === "on_your_laptop" ? "enabled" : "run_in_docker",
          )}
        />
      </DataList.Label>
      <DataList.Value>
        <Flex
          width="100%"
          align="center"
          gap="3"
          mt={{
            xs: "0",
            initial: "2",
          }}
        >
          <Switch
            size="2"
            checked={value}
            onCheckedChange={handleSwitchChange}
          />
        </Flex>
      </DataList.Value>
    </DataList.Item>
  );
};
