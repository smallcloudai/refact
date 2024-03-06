import React from "react";
import { Checkbox, Box, Grid, Text, Flex } from "@radix-ui/themes";
import { Select } from "../Select";
import { type Config } from "../../contexts/config-context";

type CapsSelectProps = {
  value: string;
  onChange: (value: string) => void;
  options: string[];
  disabled?: boolean;
};

const CapsSelect: React.FC<CapsSelectProps> = ({
  options,
  value,
  onChange,
  disabled,
}) => {
  return (
    <Flex gap="2" align="center">
      <Text size="2">Use model:</Text>
      <Select
        disabled={disabled}
        title="chat model"
        options={options}
        value={value}
        onChange={onChange}
      ></Select>
    </Flex>
  );
};

export type Checkbox = {
  name: string;
  label: string;
  checked: boolean;
  value?: string;
  disabled: boolean;
  fileName?: string;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (name: string, checked: boolean | string) => void;
  selectProps: CapsSelectProps;
  host: Config["host"];
};

export const ChatControls: React.FC<ChatControlsProps> = ({
  checkboxes,
  onCheckedChange,
  selectProps,
  host,
}) => {
  return (
    <Box pt="4" pb="4" pl="2">
      <Grid pt="4" columns="2" width="auto" gap="2">
        {Object.entries(checkboxes).map(([key, checkbox]) => {
          if (host === "web" && checkbox.name === "file_upload") {
            return null;
          }
          return (
            <Text key={key} size="2">
              <Checkbox
                size="1"
                name={checkbox.name}
                checked={checkbox.checked}
                disabled={checkbox.disabled}
                onCheckedChange={(value) => onCheckedChange(key, value)}
              />{" "}
              {checkbox.label} {checkbox.fileName}
            </Text>
          );
        })}
        <CapsSelect {...selectProps} />
      </Grid>
    </Box>
  );
};
