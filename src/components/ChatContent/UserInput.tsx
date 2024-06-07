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

function processLines(
  lines: string[],
  processedLinesMemo: JSX.Element[] = [],
): JSX.Element[] {
  if (lines.length === 0) return processedLinesMemo;

  const head = lines[0];
  const tail = lines.slice(1);
  const nextBackTicksIndex = tail.findIndex((l) => l.startsWith("```"));
  const key = `line-${processedLinesMemo.length + 1}`;

  if (!head.startsWith("```") || nextBackTicksIndex === -1) {
    const processedLines = processedLinesMemo.concat(
      <Text as="div" key={key}>
        {head}
      </Text>,
    );
    return processLines(tail, processedLines);
  }

  const endIndex = nextBackTicksIndex + 1;

  const code = [head].concat(tail.slice(0, endIndex)).join("\n");
  const processedLines = processedLinesMemo.concat(
    <Markdown key={key}>{code}</Markdown>,
  );

  const next = tail.slice(endIndex);
  return processLines(next, processedLines);
}

const ContentWithMarkdownCodeBlocks: React.FC<{ children: string }> = ({
  children,
}) => {
  const lines = children.split("\n");
  const elements = processLines(lines);
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
