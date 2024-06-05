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

  if (showTextArea) {
    return (
      <RetryForm
        onSubmit={handleSubmit}
        value={props.children}
        onClose={() => setShowTextArea(false)}
      />
    );
  }

  return (
    <Container
      minHeight="5"
      position="relative"
      onMouseEnter={() => setShowEditButton(true)}
      onMouseLeave={() => setShowEditButton(false)}
      pt="4"
    >
      <Text>
        <Markdown>{props.children}</Markdown>
      </Text>

      <Flex p="2" className={styles.footer} position="relative">
        {showEditButton && (
          <Button
            variant="ghost"
            size="1"
            onClick={() => setShowTextArea(true)}
          >
            <Pencil1Icon />
          </Button>
        )}
      </Flex>
    </Container>
  );
};
