import React from "react";
import { Text, TextProps } from "./Text";
import styles from "./Text.module.css";
import classnames from "classnames";

export const TruncateLeft: React.FC<TextProps> = ({ children, ...props }) => {
  return (
    <Text {...props} className={classnames(styles.text_rtl, props.className)}>
      &lrm;<Text className={styles.text_ltr}>{children}</Text>
    </Text>
  );
};
