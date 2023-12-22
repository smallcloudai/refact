import React from "react";
import { Section } from "@radix-ui/themes";
import styles from "./PageWrapper.module.css";

export const PageWrapper: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  return (
    <div className={styles.PageWrapper}>
      <Section
        className={styles.PageWrapper}
        width="100%"
        px={{ initial: "5", xs: "6", sm: "7", md: "9" }}
        size={{ initial: "2", md: "3" }}
        style={{ maxWidth: 858, flexGrow: 1 }}
      >
        {children}
      </Section>
    </div>
  );
};
