import React from "react";
import styles from "./Spinner.module.css";
import { Text } from "@radix-ui/themes";

export const Spinner: React.FC = () => (
  <Text asChild>
    <pre className={styles.spinner} />
  </Text>
);
