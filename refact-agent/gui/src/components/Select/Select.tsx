import React from "react";
import { Select as RadixSelect } from "@radix-ui/themes";
import styles from "./select.module.css";
import classnames from "classnames";

export type SelectProps = React.ComponentProps<typeof RadixSelect.Root> & {
  onChange: (value: string) => void;
  options: (string | ItemProps)[];
  title?: string;
  contentPosition?: "item-aligned" | "popper";
};

export type SelectRootProps = React.ComponentProps<typeof RadixSelect.Root>;
export const Root: React.FC<SelectRootProps> = RadixSelect.Root;

export type TriggerProps = React.ComponentProps<typeof RadixSelect.Trigger>;
export const Trigger: React.FC<TriggerProps> = RadixSelect.Trigger;

export type ContentProps = React.ComponentProps<typeof RadixSelect.Content>;
export const Content: React.FC<ContentProps & { className?: string }> = (
  props,
) => (
  <RadixSelect.Content
    {...props}
    className={classnames(styles.content, props.className)}
  />
);

export type ItemProps = React.ComponentProps<typeof RadixSelect.Item>;
export const Item: React.FC<ItemProps & { className?: string }> = (props) => (
  <RadixSelect.Item
    {...props}
    className={classnames(styles.item, props.className)}
  />
);

export type SeparatorProps = React.ComponentProps<typeof RadixSelect.Separator>;
export const Separator: React.FC<SeparatorProps> = RadixSelect.Separator;

export const Select: React.FC<SelectProps> = ({
  title,
  options,
  onChange,
  contentPosition,
  ...props
}) => {
  return (
    <Root {...props} onValueChange={onChange} size="1">
      <Trigger title={title} />
      <Content position={contentPosition ? contentPosition : "popper"}>
        {options.map((option, index) => {
          if (typeof option === "string") {
            return (
              <Item key={`select-item-${index}-${option}`} value={option}>
                {option}
              </Item>
            );
          }
          return (
            <Item key={`select-item-${index}-${option.value}`} {...option}>
              {option.children ?? option.textValue ?? option.value}
            </Item>
          );
        })}
      </Content>
    </Root>
  );
};
