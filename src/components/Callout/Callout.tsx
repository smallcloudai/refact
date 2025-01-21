import React, { useEffect, useState } from "react";
import {
  Flex,
  Callout as RadixCallout,
  Card,
  Text,
  Button,
} from "@radix-ui/themes";
import {
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { useTimeout } from "usehooks-ts";
import styles from "./Callout.module.css";
import classNames from "classnames";
import { useAppSelector, useLogout } from "../../hooks";
import { getIsAuthError } from "../../features/Errors/errorsSlice";

type RadixCalloutProps = React.ComponentProps<typeof RadixCallout.Root>;

export type CalloutProps = Omit<RadixCalloutProps, "onClick"> & {
  type: "info" | "error" | "warning";
  onClick?: () => void;
  timeout?: number | null;
  preventRetry?: boolean;
  hex?: string;
  message?: string | string[];
};

export const Callout: React.FC<CalloutProps> = ({
  children,
  type = "info",
  timeout = null,
  onClick = () => void 0,
  preventRetry,
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
    if (preventRetry) return;
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
  const logout = useLogout();
  const isAuthError = useAppSelector(getIsAuthError);

  return (
    <Callout
      type="error"
      color="red"
      onClick={onClick}
      timeout={timeout}
      itemType={props.itemType}
      preventRetry={isAuthError}
      {...props}
    >
      Error: {children}
      {!isAuthError && (
        <Text size="1" as="p">
          Click to retry
        </Text>
      )}
      {isAuthError && (
        <Flex as="span" gap="2" mt="3">
          <Button variant="surface" onClick={() => logout()}>
            Logout
          </Button>
          <Button asChild variant="surface" color="brown">
            <a
              href="https://discord.gg/Kts7CYg99R"
              target="_blank"
              rel="noreferrer"
            >
              Get help
            </a>
          </Button>
        </Flex>
      )}
    </Callout>
  );
};

export const InformationCallout: React.FC<Omit<CalloutProps, "type">> = ({
  timeout = null,
  onClick,
  children,
  ...props
}) => {
  return (
    <Callout
      type="info"
      color="blue"
      onClick={onClick}
      timeout={timeout}
      itemType={props.itemType}
      {...props}
    >
      Info: {children}
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
  const warningMessages = !message
    ? ["Some error occurred"]
    : Array.isArray(message)
      ? message
      : [message];

  return (
    <Callout
      type="warning"
      color={Array.isArray(message) ? "orange" : "amber"}
      onClick={onClick}
      timeout={timeout}
      {...props}
    >
      <Flex direction="column" gap="1">
        {warningMessages.map((msg, i) => (
          <span key={msg}>{i === 0 ? `Warning: ${msg}` : msg}</span>
        ))}
        {children}
      </Flex>
    </Callout>
  );
};

export const CalloutFromTop: React.FC<
  RadixCalloutProps & {
    children?: React.ReactNode;
  }
> = ({ children }) => {
  return (
    <Card asChild>
      <RadixCallout.Root color="amber" className={styles.changes_warning}>
        <Flex direction="row" align="center" gap="4" position="relative">
          <RadixCallout.Icon>
            <InfoCircledIcon />
          </RadixCallout.Icon>

          {children}
        </Flex>
      </RadixCallout.Root>
    </Card>
  );
};
