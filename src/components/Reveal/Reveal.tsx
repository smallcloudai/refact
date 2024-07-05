import React from "react";
import { Box, Button, Flex } from "@radix-ui/themes";
import styles from "./reveal.module.css";

export type RevealProps = {
  children: React.ReactNode;
  defaultOpen: boolean;
};

export const Reveal: React.FC<RevealProps> = ({ children, defaultOpen }) => {
  const [open, setOpen] = React.useState(defaultOpen);
  if (open) return <Box>{children}</Box>;
  return (
    <Button
      variant="ghost"
      onClick={() => setOpen((v) => !v)}
      asChild
      className={styles.reveal_button}
    >
      <Flex direction="column" position="relative" align="start">
        <Box className={styles.reveal_hidden}>{children}</Box>
        <Flex position="absolute" bottom="2" width="100%" justify="center">
          Click for more
        </Flex>
      </Flex>
    </Button>
  );
};
