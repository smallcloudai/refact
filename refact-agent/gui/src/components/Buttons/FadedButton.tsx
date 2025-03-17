import React from "react";
import classNames from "classnames";
import { Button, ButtonProps } from "@radix-ui/themes";
import styles from "./button.module.css";

export type FadedButtonProps = ButtonProps;

export const FadedButton: React.FC<FadedButtonProps> = (props) => {
  return (
    <Button
      variant="ghost"
      {...props}
      className={classNames(styles.button_faded, props.className)}
    />
  );
};
