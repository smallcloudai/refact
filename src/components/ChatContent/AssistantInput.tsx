import React from "react";
import { Markdown, MarkdownProps } from "../Markdown";

import { Box } from "@radix-ui/themes";

type ChatInputProps = Pick<
  MarkdownProps,
  "onNewFileClick" | "onPasteClick" | "canPaste"
> & {
  children: string;
};

export const AssistantInput: React.FC<ChatInputProps> = (props) => {
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
