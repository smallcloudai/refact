import React from "react";
import {
  Checkbox as RadixCheckbox,
  CheckboxProps as RadixCheckboxProps,
  Text,
  Flex,
} from "@radix-ui/themes";

export type CheckboxProps = RadixCheckboxProps & {
  children: React.ReactNode;
};

export const Checkbox: React.FC<CheckboxProps> = ({
  name,
  checked,
  disabled,
  onCheckedChange,
  children,
  title,
  ...props
}) => {
  return (
    <Text as="label" size="2" title={title}>
      <Flex wrap="nowrap" gap="2">
        <RadixCheckbox
          size="1"
          {...props}
          name={name}
          checked={checked}
          disabled={disabled}
          onCheckedChange={onCheckedChange}
        />
        {children}
      </Flex>
    </Text>
  );
};
