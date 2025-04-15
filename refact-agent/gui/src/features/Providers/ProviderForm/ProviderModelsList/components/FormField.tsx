import { Text, TextField } from "@radix-ui/themes";
import { FC, ReactNode } from "react";

type FormFieldProps = {
  label: string;
  defaultValue?: string;
  placeholder?: string;
  type?: TextField.RootProps["type"];
  isDisabled?: boolean;
  max?: string;
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
  children?: ReactNode;
};

/**
 * Reusable form field component with consistent styling
 */
export const FormField: FC<FormFieldProps> = ({
  label,
  defaultValue,
  placeholder,
  isDisabled,
  type,
  max,
  onChange,
  children,
}) => {
  return (
    <label>
      <Text as="div" size="2" mb="1" weight="bold">
        {label}
      </Text>
      {children ?? (
        <TextField.Root
          defaultValue={defaultValue}
          placeholder={placeholder}
          type={type}
          max={max}
          onChange={onChange}
          disabled={isDisabled}
        />
      )}
    </label>
  );
};
