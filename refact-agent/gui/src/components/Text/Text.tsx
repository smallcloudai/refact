import React from "react";
import { Text as RadixText } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./Text.module.css";

export { RadixText as Text };

export type TextProps = React.ComponentProps<typeof RadixText>;

export type SmallProps = Exclude<TextProps, "size">;

export const Small: React.FC<SmallProps> = (props) => (
  <RadixText
    {...props}
    className={classNames(styles.text_small, props.className)}
  />
);
