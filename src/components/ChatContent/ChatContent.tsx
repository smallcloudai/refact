import React, { useEffect } from "react";
import { ChatMessages } from "../../services/refact";
import { Markdown } from "../Markdown";
import { Box, Flex } from "@radix-ui/themes";

const UserInput: React.FC<{ children: string }> = (props) => {
  // TODO: retry trunciates the history up to where it was clicked, then submits
  return (<Markdown>{props.children}</Markdown>
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

export const ChatContent: React.FC<{ messages: ChatMessages }> = ({
  messages,
}) => {
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
            return <UserInput key={index}>{text}</UserInput>;
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
