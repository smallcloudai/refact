import React, { useEffect } from "react";
import { ChatMessages, isChatContextFileMessage } from "../../services/refact";
import { Markdown } from "../Markdown";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Box, Flex, Text } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";

const ContextFile: React.FC<{ name: string; children: string }> = ({
  name,
  ...props
}) => {
  return (
    <Text size="2" title={props.children}>
      <pre>📎 {name}</pre>
    </Text>
  );
};

const PlaceHolderText: React.FC = () => (
  <Text>Welcome to Refact chat! How can I assist you today?</Text>
);

const ChatInput: React.FC<{ children: string }> = (props) => {
  // TODO: new file button?
  return (
    <Box p="2" position="relative" width="100%" style={{ maxWidth: "100%" }}>
      <Markdown
        onCopyClick={(text: string) => {
          window.navigator.clipboard.writeText(text).catch(() => {
            // eslint-disable-next-line no-console
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
  isWaiting: boolean;
}> = ({ messages, onRetry, isWaiting }) => {
  const ref = React.useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollIntoView &&
      ref.current.scrollIntoView({ behavior: "instant", block: "end" });
  }, [messages]);

  return (
    <ScrollArea scrollbars="vertical">
      <Flex grow="1" direction="column" className={styles.content}>
        {messages.length === 0 && <PlaceHolderText />}
        {messages.map((message, index) => {
          if (isChatContextFileMessage(message)) {
            const [, file] = message;
            return (
              <ContextFile key={index} name={file.file_name}>
                {file.file_content}
              </ContextFile>
            );
          }

          const [role, text] = message;

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
            // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          } else if (role === "assistant") {
            return <ChatInput key={index}>{text}</ChatInput>;
          } else {
            return <Markdown key={index}>{text}</Markdown>;
          }
        })}
        {isWaiting && <Spinner />}
        <div ref={ref} />
      </Flex>
    </ScrollArea>
  );
};
