import React from "react";
import { Box } from "@radix-ui/themes";
import styles from "./PageWrapper.module.css";

export const PageWrapper: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  // TODO: this causes some weird flex issues :/
  // return (
  //   <div className={styles.PageWrapperContainer}>
  //     <Section
  //       className={styles.PageWrapper}
  //       width="100%"
  //       px={{ initial: "5", xs: "6", sm: "7", md: "9" }}
  //       size={{ initial: "2", md: "3" }}
  //       style={{ maxWidth: 858, flexGrow: 1 }}
  //     >
  //       {children}
  //     </Section>
  //   </div>
  // );
  return (
    <Box
      p={{ initial: "5", xs: "6", sm: "7", md: "9" }}
      style={{
        overflow: "hidden",
        flexGrow: 1,
        width: "100%",
        maxWidth: "100%",
      }}
      className={styles.PageWrapperContainer}
    >
      {children}
    </Box>
  );
};
