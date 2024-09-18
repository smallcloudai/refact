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
  type: "info" | "error" | "warning";
  onClick?: () => void;
  timeout?: number | null;
  hex?: string;
  message?: string | string[];
};

export const Callout: React.FC<CalloutProps> = ({
  children,
  type = "info",
  timeout = null,
  onClick = () => void 0,
  ...props
}) => {
  const [isOpened, setIsOpened] = useState(false);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      setIsOpened(true);
    }, 150);

    return () => {
      clearTimeout(timeoutId);
    };
  }, []);

  const handleRetryClick = () => {
    setIsOpened(false);
    const timeoutId = setTimeout(() => {
      onClick();
      clearTimeout(timeoutId);
    }, 300);
  };

  useTimeout(handleRetryClick, timeout);

  return (
    <RadixCallout.Root
      mx={props.mx ?? "2"}
      onClick={handleRetryClick}
      {...props}
      className={classNames(
        styles.callout_box,
        {
          [styles.callout_box_opened]: isOpened,
        },
        props.className,
      )}
    >
      {type === "warning" && <div className={styles.callout_box_background} />}
      <Flex direction="row" align="center" gap="4">
        <RadixCallout.Icon>
          {type === "error" ? <ExclamationTriangleIcon /> : <InfoCircledIcon />}
        </RadixCallout.Icon>
        <Flex direction="column" align="start" gap="1">
          <RadixCallout.Text className={styles.callout_text} wrap="wrap">
            {children}
          </RadixCallout.Text>
        </Flex>
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
      color="red"
      onClick={onClick}
      timeout={timeout}
      itemType={props.itemType}
      {...props}
    >
      Error: {children}
    </Callout>
  );
};

export const DiffWarningCallout: React.FC<Omit<CalloutProps, "type">> = ({
  timeout = null,
  onClick,
  message = null,
  children,
  ...props
}) => {
  if (!message) {
    console.log(`[DEBUG]: message is not specified`);
    return (
      <Callout
        type="warning"
        message="Some error occured"
        color="amber"
        className={
          props.itemType === "warning"
            ? styles.callout_box_background
            : undefined
        }
        onClick={onClick}
        timeout={timeout}
        // highContrast
        variant="surface"
        {...props}
      >
        Error: {children}
      </Callout>
    );
  }

  if (!Array.isArray(message)) {
    console.log(`[DEBUG]: message is not array`);
    return (
      <Callout
        type="warning"
        color="amber"
        className={
          props.itemType === "warning"
            ? styles.callout_box_background
            : undefined
        }
        onClick={onClick}
        timeout={timeout}
        // highContrast
        variant="surface"
        {...props}
      >
        Warning: {message} {children}
      </Callout>
    );
  }

  console.log(`[DEBUG]: message is array`);

  return (
    <Callout
      type="warning"
      color="orange"
      // className={
      //   props.itemType === "warning" ? styles.callout_box_background : undefined
      // }
      onClick={onClick}
      timeout={timeout}
      {...props}
    >
      <Flex direction="column" gap="1">
        {message.map((msg, i) => {
          if (i === 0) {
            return <span key={msg}>Warning: {msg}</span>;
          }
          return <span key={msg}>{msg}</span>;
        })}
        {children}
      </Flex>
    </Callout>
  );
};
