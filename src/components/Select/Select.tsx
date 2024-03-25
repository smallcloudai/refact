import React from "react";
import { Select as RadixSelect } from "@radix-ui/themes";

export type SelectProps = React.ComponentProps<typeof RadixSelect.Root> & {
  onChange: (value: string) => void;
  options: string[];
  title?: string;
};

export type SelectRootProps = React.ComponentProps<typeof RadixSelect.Root>;
export const Root: React.FC<SelectRootProps> = RadixSelect.Root;

export type TriggerProps = React.ComponentProps<typeof RadixSelect.Trigger>;
export const Trigger: React.FC<TriggerProps> = RadixSelect.Trigger;

export type ContentProps = React.ComponentProps<typeof RadixSelect.Content>;
export const Content: React.FC<ContentProps> = RadixSelect.Content;

export type ItemProps = React.ComponentProps<typeof RadixSelect.Item>;
export const Item: React.FC<ItemProps> = RadixSelect.Item;

export const Select: React.FC<SelectProps> = ({
  title,
  options,
  onChange,
  ...props
}) => {
  return (
    <RadixSelect.Root {...props} onValueChange={onChange} size="1">
      <RadixSelect.Trigger title={title} />
      <RadixSelect.Content>
        {options.map((option) => {
          return (
            <RadixSelect.Item
              key={option}
              value={option}
              style={{ overflow: "auto", overflowWrap: "anywhere" }}
            >
              {option}
            </RadixSelect.Item>
          );
        })}
      </RadixSelect.Content>
    </RadixSelect.Root>
  );
};
