import React from "react";
import { Flex, Container, Box, HoverCard, Text } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ChatContextFile } from "../../services/refact";
import classnames from "classnames";
import { TruncateLeft, Small } from "../Text";
import * as Collapsible from "@radix-ui/react-collapsible";

import { ScrollArea } from "../ScrollArea";
import ReactMarkDown from "react-markdown";

import { MarkdownCodeBlock } from "../Markdown/CodeBlock";
import { Chevron } from "../Collapsible";
import { filename } from "../../utils";

const Markdown: React.FC<{ children: string; startingLineNumber?: number }> = ({
  startingLineNumber,
  ...props
}) => {
  return (
    <ReactMarkDown
      components={{
        code(codeProps) {
          return (
            <MarkdownCodeBlock
              {...codeProps}
              showLineNumbers={true}
              startingLineNumber={startingLineNumber}
            />
          );
        },
      }}
      {...props}
    />
  );
};

function getFileInfoFromName(name: string) {
  const dot = name.lastIndexOf(".");

  if (dot === -1)
    return {
      extension: "",
      start: 1,
    };
  const extendsionAndLines = dot === -1 ? "" : name.substring(dot + 1);
  const extension = extendsionAndLines.replace(/:\d*-\d*/, "");

  if (!/:\d*-\d*/.test(extendsionAndLines)) {
    return { extension, start: 1 };
  }
  const lineIndex = extendsionAndLines.lastIndexOf(":");
  const lines = extendsionAndLines.substring(lineIndex + 1);

  const [start] = lines.split("-");
  const maybeNumber = Number(start);

  return {
    extension,
    start: maybeNumber,
  };
}

export const ContextFile: React.FC<{
  name: string;
  children: string;
  className?: string;
}> = ({ name, ...props }) => {
  const [open, setOpen] = React.useState(false);
  const { extension, start } = getFileInfoFromName(name);
  const text = "```" + extension + "\n" + props.children + "\n```";
  return (
    <Box position="relative">
      <HoverCard.Root onOpenChange={setOpen} open={open}>
        <HoverCard.Trigger>
          <Box display="inline-block">
            <Small className={classnames(styles.file, props.className)}>
              📎 <TruncateLeft>{name}</TruncateLeft>
            </Small>
          </Box>
        </HoverCard.Trigger>
        <ScrollArea scrollbars="both" asChild>
          <HoverCard.Content
            size="1"
            maxHeight="50vh"
            maxWidth="90vw"
            avoidCollisions
          >
            <Markdown startingLineNumber={start}>{text}</Markdown>
          </HoverCard.Content>
        </ScrollArea>
      </HoverCard.Root>
    </Box>
  );
};

const ContextFilesContent: React.FC<{ files: ChatContextFile[] }> = ({
  files,
}) => {
  if (files.length === 0) return null;
  return (
    <Container>
      <pre style={{ margin: 0 }}>
        <Flex wrap="nowrap" direction="column">
          {files.map((file, index) => {
            const lineText =
              file.line1 && file.line2 ? `:${file.line1}-${file.line2}` : "";
            const key = file.file_name + lineText + index;
            return (
              <ContextFile key={key} name={file.file_name + lineText}>
                {file.file_content}
              </ContextFile>
            );
          })}
        </Flex>
      </pre>
    </Container>
  );
};

export const ContextFiles: React.FC<{ files: ChatContextFile[] }> = ({
  files,
}) => {
  const [open, setOpen] = React.useState(false);

  if (files.length === 0) return null;

  const fileNames = files.map((file) => filename(file.file_name));

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="start" pb="2">
            <Text weight="light" size="1">
              🖇️ Attached {fileNames.join(", ")}
            </Text>
            <Chevron open={open} />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <ContextFilesContent files={files} />
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};
