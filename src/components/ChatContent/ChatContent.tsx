import React, { useEffect, useState } from "react";
import { ChatMessages } from "../../services/refact";
import { Markdown } from "../Markdown";
import { RightButton } from "../Buttons/Buttons";
import { Card, Box, Flex, Button, TextArea } from "@radix-ui/themes";

const UserInput: React.FC<{
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

const ContextFile: React.FC<{ children: string }> = (props) => {
  // TODO how should the context file look?
  return <Markdown>{props.children}</Markdown>;
};

const ChatInput: React.FC<{ children: string }> = (props) => {
  // TODO: new file button?
  return (
    <Box p="2" position="relative">
      <Markdown
        onCopyClick={(text: string) => {
          window.navigator.clipboard.writeText(text).catch(() => {
            console.log("failed to copy to clipboard");
          });
        }}
      >
        {props.children}
      </Markdown>
    </Box>
  );
};

export const ChatContent: React.FC<{
  messages: ChatMessages;
  onRetry: (question: ChatMessages) => void;
}> = ({ messages, onRetry }) => {
  const ref = React.useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollIntoView &&
      ref.current.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages]);

  return (
    <Flex
      grow="1"
      direction="column"
      style={{
        overflowY: "auto",
        overflowWrap: "break-word",
        wordWrap: "break-word",
      }}
    >
      {messages.map(([role, text], index) => {
        if (role === "user") {
          const handleRetry = (question: string) => {
            const toSend = messages
              .slice(0, index)
              .concat([["user", question]]);
            onRetry(toSend);
          };
          return (
            <UserInput onRetry={handleRetry} key={index}>
              {text}
            </UserInput>
          );
        } else if (role === "context_file") {
          return <ContextFile key={index}>{text}</ContextFile>;
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
        } else if (role === "assistant") {
          return <ChatInput key={index}>{text}</ChatInput>;
        } else {
          return <Markdown key={index}>{text}</Markdown>;
        }
      })}
      <div ref={ref} />
    </Flex>
  );
};
