import React, { useCallback, useMemo } from "react";
import {
  type CreateTextDocToolCall,
  type RawTextDocTool,
  ReplaceTextDocToolCall,
  TextDocToolCall,
  UpdateRegexTextDocToolCall,
  UpdateTextDocToolCall,
  isCreateTextDocToolCall,
  isReplaceTextDocToolCall,
  isUpdateRegexTextDocToolCall,
  isUpdateTextDocToolCall,
  parseRawTextDocToolCall,
} from "./types";
import { Box, Card, Flex } from "@radix-ui/themes";
import { TruncateLeft } from "../Text";
import { Link } from "../Link";
import { useEventsBusForIDE } from "../../hooks/useEventBusForIDE";
import { Markdown } from "../Markdown";
import { filename } from "../../utils/filename";
import styles from "./Texdoc.module.css";
import classNames from "classnames";
import { useCopyToClipboard } from "../../hooks/useCopyToClipboard";
import { Reveal } from "../Reveal";

export const TextDocTool: React.FC<{ toolCall: RawTextDocTool }> = ({
  toolCall,
}) => {
  const maybeTextDocToolCall = parseRawTextDocToolCall(toolCall);

  if (!maybeTextDocToolCall) return false;

  if (isCreateTextDocToolCall(maybeTextDocToolCall)) {
    return <CreateTextDoc toolCall={maybeTextDocToolCall} />;
  }

  if (isUpdateTextDocToolCall(maybeTextDocToolCall)) {
    return <UpdateTextDoc toolCall={maybeTextDocToolCall} />;
  }

  if (isReplaceTextDocToolCall(maybeTextDocToolCall)) {
    return <ReplaceTextDoc toolCall={maybeTextDocToolCall} />;
  }

  if (isUpdateRegexTextDocToolCall(maybeTextDocToolCall)) {
    return <UpdateRegexTextDoc toolCall={maybeTextDocToolCall} />;
  }

  return false;
};

const TextDocHeader: React.FC<{
  toolCall: TextDocToolCall;
}> = ({ toolCall }) => {
  const { openFile } = useEventsBusForIDE();

  // move this
  const handleOpenFile = useCallback(() => {
    if (!toolCall.function.arguments.path) return;
    openFile({ file_name: toolCall.function.arguments.path });
  }, [openFile, toolCall.function.arguments.path]);

  return (
    <Card size="1" variant="surface" mt="4" className={styles.textdoc__header}>
      <Flex gap="2" py="2" pl="2" justify="between">
        <TruncateLeft>
          <Link
            title="Open file"
            onClick={(event) => {
              event.preventDefault();
              handleOpenFile();
            }}
          >
            {toolCall.function.arguments.path}
          </Link>
        </TruncateLeft>{" "}
      </Flex>
    </Card>
  );
};

const CreateTextDoc: React.FC<{
  toolCall: CreateTextDocToolCall;
}> = ({ toolCall }) => {
  const code = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return (
      "```" + extension + "\n" + toolCall.function.arguments.content + "\n```"
    );
  }, [toolCall.function.arguments.content, toolCall.function.arguments.path]);
  const handleCopy = useCopyToClipboard();

  const lineCount = useMemo(() => code.split("\n").length, [code]);

  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9}>
        <Markdown onCopyClick={handleCopy}>{code}</Markdown>
      </Reveal>
    </Box>
  );
};

const ReplaceTextDoc: React.FC<{
  toolCall: ReplaceTextDocToolCall;
}> = ({ toolCall }) => {
  const code = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return (
      "```" +
      extension +
      "\n" +
      toolCall.function.arguments.replacement +
      "\n```"
    );
  }, [
    toolCall.function.arguments.path,
    toolCall.function.arguments.replacement,
  ]);

  const copyToClipBoard = useCopyToClipboard();
  const handleCopy = useCallback(() => {
    copyToClipBoard(toolCall.function.arguments.replacement);
  }, [copyToClipBoard, toolCall.function.arguments.replacement]);

  const lineCount = useMemo(() => code.split("\n").length, [code]);
  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9}>
        <Markdown onCopyClick={handleCopy}>{code}</Markdown>
      </Reveal>
    </Box>
  );
};

const UpdateRegexTextDoc: React.FC<{
  toolCall: UpdateRegexTextDocToolCall;
}> = ({ toolCall }) => {
  const code = useMemo(() => {
    return (
      '```py\nre.sub("' +
      toolCall.function.arguments.pattern +
      '", "' +
      toolCall.function.arguments.replacement +
      '", open("' +
      toolCall.function.arguments.path +
      '"))\n```'
    );
  }, [
    toolCall.function.arguments.path,
    toolCall.function.arguments.pattern,
    toolCall.function.arguments.replacement,
  ]);

  const lineCount = useMemo(() => code.split("\n").length, [code]);

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9}>
        <Markdown>{code}</Markdown>
      </Reveal>
    </Box>
  );
};

const UpdateTextDoc: React.FC<{
  toolCall: UpdateTextDocToolCall;
}> = ({ toolCall }) => {
  const code = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return (
      "```" +
      extension +
      "\n" +
      toolCall.function.arguments.replacement +
      "\n```"
    );
  }, [
    toolCall.function.arguments.path,
    toolCall.function.arguments.replacement,
  ]);

  const lineCount = useMemo(() => code.split("\n").length, [code]);

  return (
    <Box className={classNames(styles.textdoc, styles.textdoc__update)}>
      <TextDocHeader toolCall={toolCall} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9}>
        <Markdown useInlineStyles={false}>{code}</Markdown>
      </Reveal>
    </Box>
  );
};

function getFileExtension(filePath: string): string {
  const fileName = filename(filePath);
  if (fileName.toLocaleLowerCase().startsWith("dockerfile"))
    return "dockerfile";
  const parts = fileName.split(".");
  return parts[parts.length - 1].toLocaleLowerCase();
}
