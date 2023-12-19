import React, { useEffect } from "react";
import { ChatMessages } from "../../services/refact";
import { Markdown } from "../Markdown";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";

import { Box, Flex } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";

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
}> = ({ messages, onRetry }) => {
  const ref = React.useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollIntoView &&
      ref.current.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages]);

  return (
    <ScrollArea scrollbars="vertical">
      <Flex grow="1" direction="column" className={styles.content}>
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
    </ScrollArea>
  );
};
