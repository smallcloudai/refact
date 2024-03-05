import React from "react";
import { Collapsible } from "../Collapsible";
import { Checkbox, Box, Grid, Text } from "@radix-ui/themes";

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
};

export const ChatControls: React.FC<ChatControlsProps> = ({
  checkboxes,
  onCheckedChange,
}) => {
  return (
    <Box pt="4" pb="4" pl="2">
      <Collapsible title="Advanced: ">
        <Grid pt="4" columns="2" width="auto" gap="2">
          {Object.entries(checkboxes).map(([key, checkbox]) => {
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
        </Grid>
      </Collapsible>
    </Box>
  );
};
