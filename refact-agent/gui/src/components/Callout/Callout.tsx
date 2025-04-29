import React, { useCallback, useEffect, useState } from "react";
import {
  Flex,
  Callout as RadixCallout,
  Card,
  Text,
  Button,
  Strong,
  Em,
} from "@radix-ui/themes";
import {
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { useTimeout } from "usehooks-ts";
import styles from "./Callout.module.css";
import classNames from "classnames";
import { useAppDispatch, useAppSelector, useLogout } from "../../hooks";
import { getIsAuthError } from "../../features/Errors/errorsSlice";
import { selectBalance } from "../../features/CoinBalance";
import { dismissBalanceLowCallout } from "../../features/Errors/informationSlice";

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
  preventRetry = false,
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
    // TBD: why was this added, it won't close on click :/?
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

// TODO: Authcall out should not be generic ErrorCallout
export const ErrorCallout: React.FC<Omit<CalloutProps, "type">> = ({
  timeout = null,
  onClick,
  children,
  preventRetry,
  className,
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
      preventRetry={isAuthError ?? preventRetry}
      className={classNames(styles.callout_box_inner, className)}
      {...props}
    >
      Error: {children}
      {!isAuthError && (
        <Text size="1" as="span" className={styles.retryText}>
          {preventRetry ? "Click to close" : "Click to retry"}
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

export const BallanceCallOut: React.FC<{ onClick: () => void }> = ({
  onClick,
}) => {
  return (
    <Callout
      mt="2"
      type="error"
      color="red"
      className={classNames(styles.callout_box_inner)}
      timeout={null}
      onClick={onClick}
    >
      <Text as="p">
        💸 <Strong>Your balance is exhausted!</Strong>
      </Text>
      <Text as="p">
        You have no coins left to use Refact&apos;s AI features.
      </Text>
      <Text as="p">
        Please top up your balance or contact support if you believe this is a
        mistake.
      </Text>
      <Flex justify="end">
        <Em>Thank you for using Refact!</Em>
      </Flex>
    </Callout>
  );
};

export const BallanceLowInformation: React.FC = () => {
  const balance = useAppSelector(selectBalance);
  const dispatch = useAppDispatch();
  const handleClose = useCallback(() => {
    dispatch(dismissBalanceLowCallout());
  }, [dispatch]);

  return (
    <Callout
      type="info"
      color="blue"
      mt="2"
      timeout={null}
      onClick={handleClose}
    >
      💸 <Strong>Your balance is {balance}</Strong>
      <br />
      Please top up your balance soon or contact support if you believe this is
      a mistake.
      <br />
      <Flex as="span" justify="end">
        <Em>Thank you for using Refact!</Em>
      </Flex>
    </Callout>
  );
};
