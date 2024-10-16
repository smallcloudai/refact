import React, { useCallback } from "react";
import { Markdown } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { ToolCall } from "../../services/refact";
import { ToolContent } from "./ToolsContent";

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

type ChatInputProps = {
  message: string | null;
  toolCalls?: ToolCall[] | null;
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  message,
  toolCalls,
}) => {
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
      {message && (
        <Box py="4">
          <Markdown canHavePins={true} onCopyClick={handleCopy}>
            {message}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
    </Container>
  );
};
