import { Text, TextProps } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./AnimatedText.module.css";

export type AnimatedTextProps = TextProps & { animate?: boolean };

export const AnimatedText = ({ animate, ...props }: AnimatedTextProps) => {
  return (
    <Text
      {...props}
      className={classNames(props.className, animate && styles.animate)}
    />
  );
};
