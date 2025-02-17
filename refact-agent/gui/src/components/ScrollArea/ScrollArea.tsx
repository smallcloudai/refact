import { ScrollArea as RadixScrollArea } from "@radix-ui/themes";
import classNames from "classnames";
import React from "react";
import styles from "./ScrollArea.module.css";

export type ScrollAreaProps = React.ComponentProps<typeof RadixScrollArea> & {
  className?: string;
  scrollbars?: "vertical" | "horizontal" | "both" | undefined;
  fullHeight?: boolean;
};
export const ScrollArea = React.forwardRef<HTMLDivElement, ScrollAreaProps>(
  ({ scrollbars, className, fullHeight, ...props }, ref) => {
    const isVertical = scrollbars !== undefined && scrollbars === "vertical";

    return (
      <RadixScrollArea
        ref={ref}
        type="hover"
        {...props}
        className={classNames(
          isVertical && styles.vertical,
          fullHeight && styles.full_height,
          className,
        )}
      />
    );
  },
);

ScrollArea.displayName = "ScrollArea";
