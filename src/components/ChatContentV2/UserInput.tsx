import React, { useState } from "react";
import { Text, Container, Button, Flex } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { RetryForm } from "../ChatForm";
import styles from "./ChatContent.module.css";
import { Pencil1Icon } from "@radix-ui/react-icons";

export type UserInputProps = {
  children: string;
  onRetry: (value: string) => void;
  disableRetry?: boolean;
};

export const UserInput: React.FC<UserInputProps> = (props) => {
  const [showTextArea, setShowTextArea] = useState(false);
  const [showEditButton, setShowEditButton] = useState(false);
  const handleSubmit = (value: string) => {
    props.onRetry(value);
    setShowTextArea(false);
  };

  const handleShowTextArea = (value: boolean) => {
    setShowTextArea(value);
    setShowEditButton(false);
  };

  return (
    <Container
      position="relative"
      onMouseEnter={() => setShowEditButton(true)}
      onMouseLeave={() => setShowEditButton(false)}
      pt="4"
      // size="2"
      // align="right"
      // width="auto"
    >
      {showTextArea ? (
        <RetryForm
          onSubmit={handleSubmit}
          value={props.children}
          onClose={() => handleShowTextArea(false)}
        />
      ) : (
        <Flex
          gap="2"
          direction="column"
          display="inline-flex"
          justify="end"
          // align="end"
          // flexShrink="1"
        >
          <Text>
            <Markdown>{props.children}</Markdown>
          </Text>

          <Flex className={styles.footer} position="relative">
            {showEditButton && (
              <Button
                variant="soft"
                size="1"
                onClick={() => handleShowTextArea(true)}
              >
                <Pencil1Icon />
              </Button>
            )}
          </Flex>
        </Flex>
      )}
    </Container>
  );
};
