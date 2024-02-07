import React, { useEffect } from "react";
import {
  ChatContextFile,
  ChatMessages,
  isChatContextFileMessage,
} from "../../services/refact";
import { Markdown, MarkdownProps } from "../Markdown";
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
    <Text size="2" title={props.children} className={styles.file}>
      📎 {name}
    </Text>
  );
};

const ContextFiles: React.FC<{ files: ChatContextFile[] }> = ({ files }) => {
  return (
    <pre>
      <Flex gap="4" wrap="wrap">
        {files.map((file, index) => {
          const lineText =
            file.line1 && file.line2 ? `:${file.line1}-${file.line2}` : "";
          return (
            <ContextFile key={index} name={file.file_name + lineText}>
              {file.file_content}
            </ContextFile>
          );
        })}
      </Flex>
    </pre>
  );
};

const PlaceHolderText: React.FC = () => (
  <Text>Welcome to Refact chat! How can I assist you today?</Text>
);

type ChatInputProps = Pick<
  MarkdownProps,
  "onNewFileClick" | "onPasteClick" | "canPaste"
> & {
  children: string;
};

const ChatInput: React.FC<ChatInputProps> = (props) => {
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
        onNewFileClick={props.onNewFileClick}
        onPasteClick={props.onPasteClick}
        canPaste={props.canPaste}
      >
        {props.children}
      </Markdown>
    </Box>
  );
};

export const ChatContent: React.FC<
  {
    messages: ChatMessages;
    onRetry: (question: ChatMessages) => void;
    isWaiting: boolean;
    canPaste: boolean;
  } & Pick<MarkdownProps, "onNewFileClick" | "onPasteClick">
> = ({
  messages,
  onRetry,
  isWaiting,
  onNewFileClick,
  onPasteClick,
  canPaste,
}) => {
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
            const [, files] = message;
            return <ContextFiles key={index} files={files} />;
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
            return (
              <ChatInput
                onNewFileClick={onNewFileClick}
                onPasteClick={onPasteClick}
                canPaste={canPaste}
                key={index}
              >
                {text}
              </ChatInput>
            );
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
