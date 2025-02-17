import React from "react";
import classNames from "classnames";
import styles from "./Table.module.css";
import { useAppearance } from "../../hooks";

export const TableCell: React.FC<{
  children: React.ReactNode;
  className?: string;
  key: number;
}> = (props) => {
  const { isDarkMode } = useAppearance();

  return (
    <td
      {...props}
      className={classNames(styles.td, props.className)}
      style={{
        borderColor: isDarkMode ? "#ffffff" : "#646464",
      }}
    />
  );
};
