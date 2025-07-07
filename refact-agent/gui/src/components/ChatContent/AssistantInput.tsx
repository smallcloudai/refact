import React from "react";
import { Markdown } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { AssistantMessage, ToolCall } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { ReasoningContent } from "./ReasoningContent";
import { useCopyToClipboard } from "../../hooks";

type ChatInputProps = {
  reasoningContent?: string | null;
  toolCalls?: ToolCall[] | null;
  isLast?: boolean;
  children: AssistantMessage["ftm_content"];
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  reasoningContent,
  toolCalls,
  children,
}) => {
  const handleCopy = useCopyToClipboard();

  return (
    <Container position="relative">
      {reasoningContent && (
        <ReasoningContent
          reasoningContent={reasoningContent}
          onCopyClick={handleCopy}
        />
      )}
      {children && (
        <Box py="4">
          <Markdown canHaveInteractiveElements={true} onCopyClick={handleCopy}>
            {children}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
    </Container>
  );
};
