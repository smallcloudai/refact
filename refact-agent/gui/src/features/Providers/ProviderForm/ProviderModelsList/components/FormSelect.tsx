import { Flex, Select, Text } from "@radix-ui/themes";
import { ReactNode } from "react";

type FormSelectProps<OptionType> = {
  label: string;
  options?: OptionType[];
  optionTransformer?: (option: OptionType) => OptionType;
  value: string;
  placeholder?: string;
  description?: string;
  isDisabled?: boolean;
  onValueChange?: (value: string) => void;
  children?: ReactNode;
};

/**
 * Type for the options of the form select component
 */
export type OptionType = string | null;

/**
 * Reusable form select component with consistent styling
 */
export function FormSelect({
  label,
  options,
  value,
  placeholder,
  description,
  isDisabled,
  onValueChange,
  optionTransformer,
}: FormSelectProps<OptionType>) {
  return (
    <Flex direction="column">
      <Text as="div" size="2" mb="1" weight="bold">
        {label}
      </Text>
      {description && (
        <Text as="p" size="1" color="gray">
          {description}
        </Text>
      )}
      <Select.Root
        value={value}
        onValueChange={onValueChange}
        disabled={isDisabled}
      >
        <Select.Trigger placeholder={placeholder} />
        <Select.Content position="popper">
          {options?.map((option) => {
            if (option !== null) {
              return (
                <Select.Item key={option} value={option}>
                  {optionTransformer ? optionTransformer(option) : option}{" "}
                </Select.Item>
              );
            }
            return (
              <Select.Item key={option} value="null">
                None
              </Select.Item>
            );
          })}
        </Select.Content>
      </Select.Root>
    </Flex>
  );
}
