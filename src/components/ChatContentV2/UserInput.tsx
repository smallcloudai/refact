import React, { useState } from "react";
import { Text, Container, Button, Flex } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { RetryForm } from "../ChatForm";
import styles from "./ChatContent.module.css";

function processLines(
  lines: string[],
  processedLinesMemo: JSX.Element[] = [],
): JSX.Element[] {
  if (lines.length === 0) return processedLinesMemo;

  const [head, ...tail] = lines;
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

export type UserInputProps = {
  children: string;
  onRetry: (value: string) => void;
  disableRetry?: boolean;
};

export const UserInput: React.FC<UserInputProps> = (props) => {
  const [showTextArea, setShowTextArea] = useState(false);
  const handleSubmit = (value: string) => {
    props.onRetry(value);
    setShowTextArea(false);
  };

  const handleShowTextArea = (value: boolean) => {
    setShowTextArea(value);
  };

  const lines = props.children.split("\n");
  const elements = processLines(lines);

  return (
    <Container position="relative" pt="4">
      {showTextArea ? (
        <RetryForm
          onSubmit={handleSubmit}
          value={props.children}
          onClose={() => handleShowTextArea(false)}
        />
      ) : (
        <Flex direction="column" justify="end" align="start">
          <Button
            variant="ghost"
            size="4"
            onClick={() => handleShowTextArea(true)}
            className={styles.userInput}
            my="4"
          >
            {elements}
          </Button>
        </Flex>
      )}
    </Container>
  );
};
