import React from "react";
import { ComboboxPopover, type ComboboxStore } from "@ariakit/react";
import { Box } from "@radix-ui/themes";
import classNames from "classnames";
import { type AnchorRect } from "./utils";
import { ScrollArea } from "../ScrollArea";
import styles from "./ComboBox.module.css";

export const Popover: React.FC<
  React.PropsWithChildren & {
    store: ComboboxStore;
    hidden: boolean;
    getAnchorRect: (anchor: HTMLElement | null) => AnchorRect | null;
  }
> = ({ children, ...props }) => {
  return (
    <Box
      asChild
      className={classNames(
        "rt-PopperContent",
        "rt-HoverCardContent",
        styles.popover,
      )}
    >
      <ComboboxPopover unmountOnHide fitViewport {...props}>
        <ScrollArea scrollbars="both" className={styles.popover__scroll}>
          <Box p="1" style={{ overflowY: "hidden", overflowX: "hidden" }}>
            {children}
          </Box>
        </ScrollArea>
      </ComboboxPopover>
    </Box>
  );
};
