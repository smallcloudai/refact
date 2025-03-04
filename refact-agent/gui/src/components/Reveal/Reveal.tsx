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
  if (open)
    return (
      <Box width="100%" position="relative">
        {children}
        <Button
          variant="ghost"
          onClick={() => {
            if (defaultOpen) return;
            setOpen((v) => !v);
          }}
          asChild
          className={classNames(styles.reveal_button, {
            [styles.reveal_button_inline]: !isRevealingCode,
          })}
        >
          {!defaultOpen && (
            <Box
              className={`${styles.reveal_hidden} ${styles.reveal_hidden_exposed}`}
            >
              <Flex
                position="absolute"
                bottom="2"
                width="100%"
                justify="center"
              >
                {isRevealingCode ? (
                  "Hide details"
                ) : (
                  <Text className={styles.reveal_text}>Hide details</Text>
                )}
              </Flex>
            </Box>
          )}
        </Button>
      </Box>
    );
  return (
    <Button
      variant="ghost"
      onClick={() => {
        if (defaultOpen) return;
        setOpen((v) => !v);
      }}
      asChild
      className={classNames(styles.reveal_button, {
        [styles.reveal_button_inline]: !isRevealingCode,
      })}
    >
      <Flex direction="column" position="relative" align="start">
        <Box
          className={classNames({
            [styles.reveal_hidden]: !open,
          })}
          width="100%"
        >
          {children}
        </Box>
        {!defaultOpen && (
          <Box
            className={classNames({
              [styles.reveal_button_box]: open,
            })}
          >
            <Flex position="absolute" bottom="2" width="100%" justify="center">
              {isRevealingCode ? (
                "Click for more"
              ) : (
                <Text className={styles.reveal_text}>Click for more</Text>
              )}
            </Flex>
          </Box>
        )}
      </Flex>
    </Button>
  );
};
