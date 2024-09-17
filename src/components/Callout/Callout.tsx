import React, { useEffect, useState } from "react";
import { Flex, Callout as RadixCallout } from "@radix-ui/themes";
import {
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { useTimeout } from "usehooks-ts";
import styles from "./Callout.module.css";
import classNames from "classnames";

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
  const [isOpened, setIsOpened] = useState(false);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      setIsOpened(true);
    }, 10);

    return () => {
      clearTimeout(timeoutId);
    };
  }, []);

  const handleRetryClick = () => {
    setIsOpened(false);
    const timeoutId = setTimeout(() => {
      onClick();
      clearTimeout(timeoutId);
    }, 100);
  };

  return (
    <RadixCallout.Root
      mx="2"
      onClick={handleRetryClick}
      {...props}
      className={classNames(styles.callout_box, {
        [styles.callout_box_opened]: isOpened,
      })}
    >
      <Flex direction="row" align="center" gap="4">
        <RadixCallout.Icon>
          {type === "error" ? <ExclamationTriangleIcon /> : <InfoCircledIcon />}
        </RadixCallout.Icon>
        <RadixCallout.Text className={styles.callout_text} wrap="wrap">
          {children}
        </RadixCallout.Text>
      </Flex>
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
      color={props.itemType === "warning" ? "orange" : "red"}
      onClick={onClick}
      timeout={timeout}
      {...props}
    >
      Warning: {children}
    </Callout>
  );
};
