import React, { useCallback, useMemo } from "react";
import { Markdown } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { ToolCall } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { useAppSelector, useEventsBusForIDE } from "../../hooks";
import { selectActiveFile } from "../../features/Chat/activeFile";
import { selectSelectedSnippet } from "../../features/Chat";

type ChatInputProps = {
  message: string | null;
  toolCalls?: ToolCall[] | null;
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

export const AssistantInput: React.FC<ChatInputProps> = ({
  message,
  toolCalls,
}) => {
  const activeFile = useAppSelector(selectActiveFile);

  const snippet = useAppSelector(selectSelectedSnippet);

  const codeLineCount = useMemo(() => {
    if (snippet.code.length === 0) return 0;
    return snippet.code.split("\n").filter((str) => str).length;
  }, [snippet.code]);

  const { newFile, diffPasteBack } = useEventsBusForIDE();
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
          <Markdown
            onCopyClick={handleCopy}
            onNewFileClick={newFile}
            onPasteClick={diffPasteBack}
            canPaste={activeFile.can_paste && codeLineCount > 0}
            canHavePins={true}
          >
            {message}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
    </Container>
  );
};
