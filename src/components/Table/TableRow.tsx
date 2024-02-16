import React from "react";
import styles from "./Table.module.css";

export const TableRow: React.FC<{ children: React.ReactNode; key: string }> = (
  props,
) => <tr {...props} className={styles.tr} />;
