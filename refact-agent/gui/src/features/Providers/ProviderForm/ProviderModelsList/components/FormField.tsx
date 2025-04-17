import { Text, TextField } from "@radix-ui/themes";
import { FC, ReactNode } from "react";
import { Markdown } from "../../../../../components/Markdown";

type FormFieldProps = {
  label: string;
  value?: string;
  placeholder?: string;
  description?: string;
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
  value,
  placeholder,
  description,
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
      {description && (
        <Text as="div" size="1" color="gray" my="1">
          <Markdown>{description}</Markdown>
        </Text>
      )}
      {children ?? (
        <TextField.Root
          value={value}
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
