import { Text, TextProps } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./AnimatedText.module.css";

export type AnimatedTextProps = TextProps & { animating?: boolean };

export const AnimatedText = ({ animating, ...props }: AnimatedTextProps) => {
  return (
    <Text
      {...props}
      className={classNames(props.className, animating && styles.animate)}
    />
  );
};
