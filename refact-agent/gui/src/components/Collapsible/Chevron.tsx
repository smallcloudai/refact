import React from "react";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./Chevron.module.css";

export type ChevronProps = {
  open: boolean;
  className?: string;
  isUpDownChevron?: boolean;
};

export const Chevron: React.FC<ChevronProps> = ({
  open,
  className,
  isUpDownChevron = false,
}) => {
  return (
    <ChevronDownIcon
      className={classNames(
        {
          [styles.down]: open,
          [styles.right]: !open && !isUpDownChevron,
          [styles.up]: !open && isUpDownChevron,
        },
        className,
      )}
      style={{ minWidth: 16, minHeight: 16 }}
    />
  );
};
