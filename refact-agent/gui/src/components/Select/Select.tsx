import React, { ReactNode, useMemo } from "react";
import { HoverCard, Select as RadixSelect } from "@radix-ui/themes";
import styles from "./select.module.css";
import classnames from "classnames";

type SeparatorOption = { type: "separator"; key?: string };
function isSeparator(option: unknown): option is SeparatorOption {
  if (!option) return false;
  if (typeof option !== "object") return false;
  if (!("type" in option)) return false;
  return option.type === "separator";
}
export type SelectProps = React.ComponentProps<typeof RadixSelect.Root> & {
  onChange: (value: string) => void;
  options: (string | ItemProps | SeparatorOption)[];
  title?: string;
  contentPosition?: "item-aligned" | "popper";
  value?: string;
  disabled?: boolean;
  open?: SelectRootProps["open"];
  defaultOpen?: SelectRootProps["defaultOpen"];
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

export type ItemProps = React.ComponentProps<typeof RadixSelect.Item> & {
  tooltip?: ReactNode;
};

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
  const [isOpen, setIsOpen] = React.useState(
    props.open ?? props.defaultOpen ?? false,
  );
  const maybeSelectedOption = useMemo(() => {
    if (typeof props.value === "undefined") return null;
    const selectOption = options.find(
      (option) =>
        typeof option !== "string" &&
        "value" in option &&
        option.value === props.value,
    );
    if (!selectOption) return null;
    if (typeof selectOption === "string") return null;
    return selectOption;
  }, [props.value, options]);

  return (
    <Root {...props} onValueChange={onChange} onOpenChange={setIsOpen} size="1">
      {maybeSelectedOption &&
      "tooltip" in maybeSelectedOption &&
      maybeSelectedOption.tooltip &&
      !isOpen ? (
        <HoverCard.Root openDelay={1000}>
          <HoverCard.Trigger>
            <Trigger />
          </HoverCard.Trigger>
          <HoverCard.Content size="1" side="top">
            {maybeSelectedOption.tooltip}
          </HoverCard.Content>
        </HoverCard.Root>
      ) : (
        <Trigger title={title} />
      )}
      <Content position={contentPosition ?? "popper"}>
        {options.map((option, index) => {
          if (typeof option === "string") {
            return (
              <Item key={`select-item-${index}-${option}`} value={option}>
                {option}
              </Item>
            );
          }
          if (isSeparator(option)) {
            return <Separator key={option.key ?? `separator-${index}`} />;
          }
          if (option.tooltip) {
            return (
              <Item key={`select-item-${index}-${option.value}`} {...option}>
                <HoverCard.Root openDelay={1000}>
                  <HoverCard.Trigger>
                    <div>
                      <span className={styles.trigger_only}>
                        {option.textValue ?? option.value}
                      </span>
                      <span className={styles.dropdown_only}>
                        {option.children}
                      </span>
                    </div>
                  </HoverCard.Trigger>
                  <HoverCard.Content size="1">
                    {option.tooltip}
                  </HoverCard.Content>
                </HoverCard.Root>
              </Item>
            );
          }
          return (
            <Item key={`select-item-${index}-${option.value}`} {...option}>
              <span className={styles.trigger_only}>
                {option.textValue ?? option.value}
              </span>
              <span className={styles.dropdown_only}>{option.children}</span>
            </Item>
          );
        })}
      </Content>
    </Root>
  );
};
