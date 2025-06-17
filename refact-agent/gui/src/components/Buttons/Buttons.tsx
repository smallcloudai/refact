import React, { forwardRef, useCallback } from "react";
import { IconButton, Button, Flex } from "@radix-ui/themes";
import {
  PaperPlaneIcon,
  ExitIcon,
  Cross1Icon,
  FileTextIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./button.module.css";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import { useAppSelector } from "../../hooks";
import { selectApiKey } from "../../features/Config/configSlice";
import { PuzzleIcon } from "../../images/PuzzleIcon";

type IconButtonProps = React.ComponentProps<typeof IconButton>;
type ButtonProps = React.ComponentProps<typeof Button>;

export const PaperPlaneButton: React.FC<IconButtonProps> = (props) => (
  <IconButton variant="ghost" {...props}>
    <PaperPlaneIcon />
  </IconButton>
);
export const AgentIntegrationsButton = forwardRef<
  HTMLButtonElement,
  IconButtonProps
>((props, ref) => (
  <IconButton variant="ghost" {...props} ref={ref}>
    <PuzzleIcon />
  </IconButton>
));

AgentIntegrationsButton.displayName = "AgentIntegrationsButton";

export const ThreadHistoryButton: React.FC<IconButtonProps> = (props) => (
  <IconButton variant="ghost" {...props}>
    <FileTextIcon />
  </IconButton>
);

export const BackToSideBarButton: React.FC<IconButtonProps> = (props) => (
  <IconButton variant="ghost" {...props}>
    <ExitIcon style={{ transform: "scaleX(-1)" }} />
  </IconButton>
);

export const CloseButton: React.FC<
  IconButtonProps & { iconSize?: number | string }
> = ({ iconSize, ...props }) => (
  <IconButton variant="ghost" {...props}>
    <Cross1Icon width={iconSize} height={iconSize} />
  </IconButton>
);

export const RightButton: React.FC<ButtonProps & { className?: string }> = (
  props,
) => {
  return (
    <Button
      size="1"
      variant="surface"
      {...props}
      className={classNames(styles.rightButton, props.className)}
    />
  );
};

type FlexProps = React.ComponentProps<typeof Flex>;

export const RightButtonGroup: React.FC<React.PropsWithChildren & FlexProps> = (
  props,
) => {
  return (
    <Flex
      {...props}
      gap="1"
      className={classNames(styles.rightButtonGroup, props.className)}
    />
  );
};

type AgentUsageLinkButtonProps = ButtonProps & {
  href?: string;
  onClick?: () => void;
  target?: HTMLFormElement["target"];
  isPlanFree?: boolean;
  children?: React.ReactNode;
  disabled?: boolean;
};

const SUBSCRIPTION_URL =
  // "https://refact.smallcloud.ai/refact/update-subscription";
  "http://127.0.0.1:8008/my-workspace";

// const SUBSCRIPTION_FALLBACK_URL = "https://refact.smallcloud.ai/";
const SUBSCRIPTION_FALLBACK_URL = "http://127.0.0.1:8008/";

export const AgentUsageLinkButton: React.FC<AgentUsageLinkButtonProps> = ({
  href,
  isPlanFree,
  children,
  onClick,
  disabled,
  ...rest
}) => {
  const openUrl = useOpenUrl();
  const apiKey = useAppSelector(selectApiKey);
  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const fetchSubscriptionUrl = useCallback(async (): Promise<string | null> => {
    try {
      const response = await fetch(SUBSCRIPTION_URL, {
        method: "GET",
        headers: {
          Authorization: `Bearer ${apiKey}`,
        },
      });

      if (!response.ok) {
        openUrl(SUBSCRIPTION_FALLBACK_URL);
        return null;
      }

      const data = (await response.json()) as { url: string };
      return data.url;
    } catch (e) {
      openUrl(SUBSCRIPTION_FALLBACK_URL);
      return null;
    }
  }, [apiKey, openUrl]);

  const handleClick = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();

      if (isLoading) return;

      try {
        setIsLoading(true);
        setError(null);

        if (href && isPlanFree) {
          openUrl(href);
        } else if (isPlanFree !== undefined && !isPlanFree) {
          const url = await fetchSubscriptionUrl();
          if (url) {
            openUrl(url);
          }
        }

        onClick?.();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error("Error in LinkButton:", err);
        setError(err instanceof Error ? err.message : "An error occurred");
      } finally {
        setIsLoading(false);
      }
    },
    [href, isPlanFree, onClick, openUrl, fetchSubscriptionUrl, isLoading],
  );

  return (
    <form onSubmit={(event) => void handleClick(event)}>
      <Button type="submit" disabled={disabled ?? isLoading} {...rest}>
        {isLoading ? "Loading..." : children}
      </Button>
      {error && <div className={styles.error}>{error}</div>}
    </form>
  );
};
