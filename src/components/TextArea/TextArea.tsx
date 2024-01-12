import React from "react";
import { TextArea as RadixTextArea } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./TextArea.module.css";

export type TextAreaProps = React.ComponentProps<typeof RadixTextArea> & {
  className?: string;
  value?: string;
};

export const TextArea = React.forwardRef<HTMLTextAreaElement, TextAreaProps>(
  (props, ref) => {
    return (
      <RadixTextArea
        {...props}
        className={classNames(styles.textarea, props.className)}
        ref={ref}
      />
    );
  },
);

TextArea.displayName = "TextArea";
