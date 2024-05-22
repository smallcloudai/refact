import React, { useEffect, useImperativeHandle } from "react";
import { ChatMessages, isChatContextFileMessage } from "../../services/refact";
import { MarkdownProps } from "../Markdown";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Text } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
// import { SystemInput } from "./SystemInput";

const PlaceHolderText: React.FC = () => (
  <Text>Welcome to Refact chat! How can I assist you today?</Text>
);

export type ChatContentProps = {
  messages: ChatMessages;
  onRetry: (question: ChatMessages) => void;
  isWaiting: boolean;
  canPaste: boolean;
  isStreaming: boolean;
} & Pick<MarkdownProps, "onNewFileClick" | "onPasteClick">;

export const ChatContent = React.forwardRef<HTMLDivElement, ChatContentProps>(
  (props, ref) => {
    const {
      messages,
      onRetry,
      isWaiting,
      onNewFileClick,
      onPasteClick,
      canPaste,
      isStreaming,
    } = props;

    const innerRef = React.useRef<HTMLDivElement>(null);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    useImperativeHandle(ref, () => innerRef.current!, []);

    useEffect(() => {
      innerRef.current?.scrollIntoView &&
        innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    }, [messages]);

    return (
      <ScrollArea style={{ flexGrow: 1, height: "auto" }} scrollbars="vertical">
        <Flex direction="column" className={styles.content} px="1">
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
                <UserInput
                  onRetry={handleRetry}
                  key={index}
                  disableRetry={isStreaming || isWaiting}
                >
                  {text}
                </UserInput>
              );
            } else if (role === "assistant") {
              return (
                <AssistantInput
                  onNewFileClick={onNewFileClick}
                  onPasteClick={onPasteClick}
                  canPaste={canPaste}
                  key={index}
                >
                  {text}
                </AssistantInput>
              );
              // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
            } else {
              return null;
              // return <SystemInput key={index}>{text}</SystemInput>;
            }
          })}
          {isWaiting && <Spinner />}
          <div ref={innerRef} />
        </Flex>
      </ScrollArea>
    );
  },
);

ChatContent.displayName = "ChatContent";
