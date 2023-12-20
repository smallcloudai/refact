import React from "react";
import { Select as RadixSelect } from "@radix-ui/themes";

type SelectProps = {
  onChange: (event: React.ChangeEvent<HTMLSelectElement>) => void;
  options: string[];
  defaultValue?: string;
  value?: string;
  label?: string;
  title?: string;
};

export const Select: React.FC<SelectProps> = ({ title, options, ...props }) => {
  return (
    <RadixSelect.Root {...props}>
      <RadixSelect.Trigger title={title} />
      <RadixSelect.Content>
        {options.map((option) => {
          return (
            <RadixSelect.Item key={option} value={option}>
              {option}
            </RadixSelect.Item>
          );
        })}
      </RadixSelect.Content>
    </RadixSelect.Root>
  );
};
