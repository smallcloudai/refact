import { ScrollArea as RadixScrollArea } from "@radix-ui/themes";
import classNames from "classnames";
import React from "react";
import styles from "./ScrollArea.module.css";

type ScrollAreaProps = React.ComponentProps<typeof RadixScrollArea>;
export const ScrollArea: React.FC<
  ScrollAreaProps & {
    className?: string;
  }
> = (props) => {
  return (
    <RadixScrollArea
      {...props}
      className={classNames(styles.scrollArea, props.className)}
    />
  );
};
