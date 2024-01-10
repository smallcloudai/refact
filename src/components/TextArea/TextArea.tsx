import React from "react";
import { TextArea as RadixTextArea } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./TextArea.module.css";

export type TextAreaProps = React.ComponentProps<typeof RadixTextArea> & {
  className?: string;
};

export const TextArea: React.FC<TextAreaProps> = (props) => {
  return (
    <RadixTextArea
      {...props}
      className={classNames(styles.textarea, props.className)}
    />
  );
};
