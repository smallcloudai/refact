import { ScrollArea as RadixScrollArea } from "@radix-ui/themes";
import classNames from "classnames";
import React from "react";
import styles from "./ScrollArea.module.css";

export type ScrollAreaProps = React.ComponentProps<typeof RadixScrollArea> & {
  className?: string;
  scrollbars?: "vertical" | "horizontal" | "both" | undefined;
};
export const ScrollArea: React.FC<ScrollAreaProps> = ({
  scrollbars,
  className,
  ...props
}) => {
  const isVertical = scrollbars !== undefined && scrollbars === "vertical";

  return (
    <RadixScrollArea
      type="hover"
      {...props}
      className={classNames(isVertical && styles.vertical, className)}
    />
  );
};
