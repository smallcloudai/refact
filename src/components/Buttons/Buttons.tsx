import React from "react";
import { IconButton, Button, Flex } from "@radix-ui/themes";
import { PaperPlaneIcon, ExitIcon, Cross1Icon } from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./button.module.css";

type IconButtonProps = React.ComponentProps<typeof IconButton>;
type ButtonProps = React.ComponentProps<typeof Button>;

export const PaperPlaneButton: React.FC<IconButtonProps> = (props) => (
  <IconButton variant="ghost" {...props}>
    <PaperPlaneIcon />
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

export const RightButtonGroup: React.FC<
  React.PropsWithChildren & {
    className?: string;
    direction?: "row" | "column";
  }
> = (props) => {
  return (
    <Flex
      {...props}
      gap="1"
      className={classNames(styles.rightButtonGroup, props.className)}
    />
  );
};
