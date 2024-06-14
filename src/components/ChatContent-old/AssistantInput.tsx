import React from "react";
import { Markdown, MarkdownProps } from "../Markdown";

import { Box } from "@radix-ui/themes";

type ChatInputProps = Pick<
  MarkdownProps,
  "onNewFileClick" | "onPasteClick" | "canPaste"
> & {
  children: string;
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
  return (
    <Box p="2" position="relative" width="100%" style={{ maxWidth: "100%" }}>
      <Markdown
        onCopyClick={(text: string) => {
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          if (window.navigator?.clipboard?.writeText) {
            window.navigator.clipboard.writeText(text).catch(() => {
              // eslint-disable-next-line no-console
              console.log("failed to copy to clipboard");
            });
          } else {
            fallbackCopying(text);
          }
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
