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
};

const ContentWithMarkdownCodeBlocks: React.FC<{ children: string }> = ({
  children,
}) => {
  const elements: JSX.Element[] = [];
  const lines = children.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith("```")) {
      // no need to add a new line
      const rest = lines.slice(i + 1);
      const nextIndex = rest.findIndex((l) => l.startsWith("```"));
      if (nextIndex !== -1) {
        const endIndex = i + 1 + nextIndex;
        const code = lines.slice(i, endIndex).join("\n");
        elements.push(
          <Markdown key={`codeblock-${i}:${endIndex}`}>{code}</Markdown>,
        );
        i = endIndex;
      } else {
        elements.push(<Text key={"unterminated-backticks-" + i}>{line}</Text>);
      }
    } else {
      elements.push(
        <Text key={"text-" + i} as="div">
          {line}
        </Text>,
      );
    }
  }

  return <Box py="4">{elements}</Box>;
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
        <RightButton
          className={styles.retryButton}
          title="retry"
          onClick={() => setShowTextArea(true)}
          disabled={props.disableRetry}
        >
          Retry
        </RightButton>

        <ContentWithMarkdownCodeBlocks>
          {props.children}
        </ContentWithMarkdownCodeBlocks>
      </Box>
    </Card>
  );
};
