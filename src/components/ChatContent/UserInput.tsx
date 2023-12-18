import React, { useState } from "react";
import { RightButton } from "../Buttons/Buttons";
import { Card, Button, TextArea } from "@radix-ui/themes";
import { Markdown } from "../Markdown";

export const UserInput: React.FC<{
  children: string;
  onRetry: (value: string) => void;
}> = (props) => {
  // retry truncates the history up to where it was clicked
  const [showTextArea, setShowTextArea] = useState(false);

  const toggleTextArea = () => setShowTextArea((last) => !last);
  const [value, onChange] = useState(props.children);
  const closeAndReset = () => {
    onChange(props.children);
    toggleTextArea();
  };

  const handleRetry = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0) {
      props.onRetry(trimmedValue);
      closeAndReset();
    }
  };

  if (showTextArea) {
    return (
      <form
        onSubmit={(event) => {
          event.preventDefault();
          handleRetry();
        }}
      >
        <TextArea
          value={value}
          onChange={(event) => onChange(event.target.value)}
        />
        <Button type="submit">Submit</Button>
        <Button onClick={closeAndReset}>Cancel</Button>
      </form>
    );
  }

  return (
    <Card variant="classic">
      <RightButton onClick={toggleTextArea}>Retry</RightButton>
      <Markdown>{props.children}</Markdown>
    </Card>
  );
};
