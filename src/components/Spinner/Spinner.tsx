import React from "react";
import styles from "./Spinner.module.css";
import { Text } from "@radix-ui/themes";
import classNames from "classnames";

export type SpinnerProps = {
  spinning: boolean;
};

export const Spinner: React.FC<SpinnerProps> = ({ spinning }) => (
  <Text asChild>
    <pre className={classNames(styles.spinner, spinning && styles.spinning)} />
  </Text>
);
