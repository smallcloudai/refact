import React from "react";
import { Box, Button, Flex, Text } from "@radix-ui/themes";
import styles from "./reveal.module.css";
import classNames from "classnames";

export type RevealProps = {
  children: React.ReactNode;
  defaultOpen: boolean;
  isRevealingCode?: boolean;
};

export const Reveal: React.FC<RevealProps> = ({
  children,
  defaultOpen,
  isRevealingCode = false,
}) => {
  const [open, setOpen] = React.useState(defaultOpen);
  if (open) return <Box width="100%">{children}</Box>;
  return (
    <Button
      variant="ghost"
      onClick={() => setOpen((v) => !v)}
      asChild
      className={classNames(styles.reveal_button, {
        [styles.reveal_button_inline]: !isRevealingCode,
      })}
    >
      <Flex direction="column" position="relative" align="start">
        <Box className={styles.reveal_hidden} width="100%">
          {children}
        </Box>
        <Flex position="absolute" bottom="2" width="100%" justify="center">
          {isRevealingCode ? (
            "Click for more"
          ) : (
            <Text className={styles.reveal_text}>Click for more</Text>
          )}
        </Flex>
      </Flex>
    </Button>
  );
};
