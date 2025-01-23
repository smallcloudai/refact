import React, { useCallback } from "react";
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

type IconButtonProps = React.ComponentProps<typeof IconButton>;
type ButtonProps = React.ComponentProps<typeof Button>;

export const PaperPlaneButton: React.FC<IconButtonProps> = (props) => (
  <IconButton variant="ghost" {...props}>
    <PaperPlaneIcon />
  </IconButton>
);

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

export const LinkButton: React.FC<
  ButtonProps & {
    href?: string;
    target?: HTMLFormElement["target"];
    onClick?: () => void;
  }
> = ({ href, target, onClick, ...rest }) => {
  const openUrl = useOpenUrl();
  const handleClick = useCallback(() => {
    if (onClick) onClick();
    if (href) openUrl(href);
  }, [href, onClick, openUrl]);
  return (
    <form action={href} target={target} onSubmit={handleClick}>
      <Button type="submit" {...rest}>
        Upgrade to our pro plan
      </Button>
    </form>
  );
};
