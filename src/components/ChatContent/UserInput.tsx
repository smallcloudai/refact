import React, { useState } from "react";
import { RightButton } from "../Buttons/Buttons";
import { Card, Box, Text } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { RetryForm } from "../ChatForm";
import styles from "./ChatContent.module.css";

export type UserInputProps = {
  children: string;
  onRetry: (value: string) => void;
  disableRetry?: boolean;
  canRetry?: boolean;
};

export const UserInput: React.FC<UserInputProps> = (props) => {
  const [showTextArea, setShowTextArea] = useState(false);
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
    <Card
      variant="classic"
      m="1"
      style={{
        wordWrap: "break-word",
        wordBreak: "break-word",
        whiteSpace: "break-spaces",
      }}
    >
      <Box style={{ minHeight: "var(--space-5)" }}>
        {props.canRetry && (
          <RightButton
            className={styles.retryButton}
            title="retry"
            onClick={() => setShowTextArea(true)}
            disabled={props.disableRetry}
          >
            Retry
          </RightButton>
        )}

        <Box py="4">
          <Text>
            <Markdown>{props.children}</Markdown>
          </Text>
        </Box>
      </Box>
    </Card>
  );
};
