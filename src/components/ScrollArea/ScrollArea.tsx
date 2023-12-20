import { ScrollArea as RadixScrollArea } from "@radix-ui/themes";
import classNames from "classnames";
import React from "react";
import styles from "./ScrollArea.module.css";

type ScrollAreaProps = React.ComponentProps<typeof RadixScrollArea>;
export const ScrollArea: React.FC<
  ScrollAreaProps & {
    className?: string;
    scrollbars?: "vertical" | "horizontal" | "both" | undefined;
  }
> = ({ scrollbars, className, ...props }) => {
  const isVertical = scrollbars !== undefined && scrollbars === "vertical";

  return (
    <RadixScrollArea
      type="hover"
      {...props}
      className={classNames(isVertical && styles.vertical, className)}
    />
  );
};
