import React, { useCallback } from "react";
import { Markdown, MarkdownProps } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { ToolCall, ToolResult } from "../../services/refact";
import { ToolContent } from "./ToolsContent";

type ChatInputProps = Pick<
  MarkdownProps,
  "onNewFileClick" | "onPasteClick" | "canPaste"
> & {
  message: string | null;
  toolCalls?: ToolCall[] | null;
  toolResults: Record<string, ToolResult>;
};

function fallbackCopying(text: string) {
  const textArea = document.createElement("textarea");
  textArea.value = text;

  textArea.style.top = "0";
  textArea.style.left = "0";
  textArea.style.position = "fixed";

  document.body.appendChild(textArea);
  textArea.focus();
  textArea.select();

  document.execCommand("copy");
  document.body.removeChild(textArea);
}

export const AssistantInput: React.FC<ChatInputProps> = (props) => {
  const handleCopy = useCallback((text: string) => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (window.navigator?.clipboard?.writeText) {
      window.navigator.clipboard.writeText(text).catch(() => {
        // eslint-disable-next-line no-console
        console.log("failed to copy to clipboard");
      });
    } else {
      fallbackCopying(text);
    }
  }, []);

  return (
    <Container position="relative">
      {props.message && (
        <Box py="4">
          <Markdown
            onCopyClick={handleCopy}
            onNewFileClick={props.onNewFileClick}
            onPasteClick={props.onPasteClick}
            canPaste={props.canPaste}
          >
            {props.message}
          </Markdown>
        </Box>
      )}
      {props.toolCalls && (
        <ToolContent toolCalls={props.toolCalls} results={props.toolResults} />
      )}
    </Container>
  );
};
