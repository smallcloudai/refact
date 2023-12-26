import React from "react";
import { Callout as RadixCallout } from "@radix-ui/themes";
import {
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { useTimeout } from "usehooks-ts";

type RadixCalloutProps = React.ComponentProps<typeof RadixCallout.Root>;

export type CalloutProps = Omit<RadixCalloutProps, "onClick"> & {
  type: "info" | "error";
  onClick?: () => void;
  timeout?: number | null;
};

export const Callout: React.FC<CalloutProps> = ({
  children,
  type = "info",
  timeout = null,
  onClick = () => void 0,
  ...props
}) => {
  useTimeout(onClick, timeout);
  return (
    <RadixCallout.Root onClick={onClick} {...props}>
      <RadixCallout.Icon>
        {type === "error" ? <ExclamationTriangleIcon /> : <InfoCircledIcon />}
      </RadixCallout.Icon>
      <RadixCallout.Text>{children}</RadixCallout.Text>
    </RadixCallout.Root>
  );
};

export const ErrorCallout: React.FC<Omit<CalloutProps, "type">> = ({
  timeout = null,
  onClick,
  children,
  ...props
}) => {
  return (
    <Callout
      type="error"
      color="red"
      onClick={onClick}
      timeout={timeout}
      {...props}
    >
      Error: {children}
    </Callout>
  );
};
