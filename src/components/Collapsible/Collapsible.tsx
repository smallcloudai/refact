import React from "react";
import * as RadixCollapsible from "@radix-ui/react-collapsible";
import { Cross2Icon, RowSpacingIcon } from "@radix-ui/react-icons";
import { Flex, Button } from "@radix-ui/themes";

export type CollapsibleProps = Pick<
  RadixCollapsible.CollapsibleProps,
  "disabled" | "className"
> &
  React.PropsWithChildren<{
    className?: string;
    disabled?: boolean;
    title?: string;
  }>;

export const Collapsible: React.FC<CollapsibleProps> = ({
  children,
  title,
  ...props
}) => {
  const [open, setOpen] = React.useState(false);
  return (
    <RadixCollapsible.Root
      {...props}
      className={props.className}
      open={open}
      onOpenChange={setOpen}
    >
      <Flex align="center" justify="between">
        <RadixCollapsible.Trigger asChild>
          <Button variant="ghost">
            {title}
            {open ? <Cross2Icon /> : <RowSpacingIcon />}
          </Button>
        </RadixCollapsible.Trigger>
      </Flex>

      <RadixCollapsible.Content>{children}</RadixCollapsible.Content>
    </RadixCollapsible.Root>
  );
};
