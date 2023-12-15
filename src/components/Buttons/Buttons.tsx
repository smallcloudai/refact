import React from "react";
import { IconButton, Button } from "@radix-ui/themes";
import { PaperPlaneIcon, ExitIcon } from "@radix-ui/react-icons";
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

export const RightButton: React.FC<ButtonProps & { className?: string }> = (
  props,
) => {
  return (
    <Button
      {...props}
      size="1"
      variant="surface"
      className={classNames(styles.rightButton, props.className)}
    />
  );
};
