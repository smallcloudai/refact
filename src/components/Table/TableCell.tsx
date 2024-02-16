import React from "react";
import classNames from "classnames";
import styles from "./Table.module.css";

export const TableCell: React.FC<{
  children: React.ReactNode;
  className?: string;
  key: string;
}> = (props) => (
  <td {...props} className={classNames(styles.td, props.className)} />
);
