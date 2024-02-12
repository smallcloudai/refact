import React, { useState } from "react";
import { RightButton } from "../Buttons/Buttons";
import { Card, Box } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { RetryForm } from "../ChatForm";

export const UserInput: React.FC<{
  children: string;
  onRetry: (value: string) => void;
}> = (props) => {
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
        <RightButton title="retry" onClick={() => setShowTextArea(true)}>
          Retry
        </RightButton>
        <Markdown>{props.children}</Markdown>
      </Box>
    </Card>
  );
};
