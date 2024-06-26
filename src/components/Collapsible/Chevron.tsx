import React from "react";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./Chevron.module.css";

export type ChevronProps = {
  open: boolean;
  className?: string;
};

export const Chevron: React.FC<ChevronProps> = ({ open, className }) => {
  return (
    <ChevronDownIcon className={classNames(open && styles.up, className)} />
  );
};
