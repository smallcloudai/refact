import React, {
  forwardRef,
  useCallback,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  type CreateTextDocToolCall,
  type RawTextDocTool,
  ReplaceTextDocToolCall,
  TextDocToolCall,
  UpdateRegexTextDocToolCall,
  UpdateTextDocToolCall,
  UpdateTextDocByLinesToolCall,
  isCreateTextDocToolCall,
  isReplaceTextDocToolCall,
  isUpdateRegexTextDocToolCall,
  isUpdateTextDocToolCall,
  isUpdateTextDocByLinesToolCall,
  parseRawTextDocToolCall,
} from "./types";
import { Box, Card, Flex, Button } from "@radix-ui/themes";
import { TruncateLeft } from "../Text";
import { Link } from "../Link";
import { filename } from "../../utils/filename";
import styles from "./Texdoc.module.css";
import { useCopyToClipboard } from "../../hooks/useCopyToClipboard";
import { Reveal } from "../Reveal";
import { useAppSelector, useHideScroll, useEventsBusForIDE } from "../../hooks";
import { selectCanPaste, selectChatId } from "../../features/Chat";
import { toolsApi } from "../../services/refact";
import { ErrorCallout } from "../Callout";
import { isRTKResponseErrorWithDetailMessage } from "../../utils";
import { MarkdownCodeBlock } from "../Markdown/CodeBlock";
import classNames from "classnames";

export const TextDocTool: React.FC<{
  toolCall: RawTextDocTool;
  toolFailed?: boolean;
}> = ({ toolCall, toolFailed = false }) => {
  if (toolFailed) return false;

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

  if (isUpdateTextDocByLinesToolCall(maybeTextDocToolCall)) {
    return <UpdateTextDocByLines toolCall={maybeTextDocToolCall} />;
  }

  return false;
};

type TextDocHeaderProps = { toolCall: TextDocToolCall };
const TextDocHeader = forwardRef<HTMLDivElement, TextDocHeaderProps>(
  ({ toolCall }, ref) => {
    const { queryPathThenOpenFile, diffPasteBack, sendToolCallToIde } =
      useEventsBusForIDE();
    const [requestDryRun, dryRunResult] =
      toolsApi.useDryRunForEditToolMutation();
    const [errorMessage, setErrorMessage] = useState<string>("");
    const canPaste = useAppSelector(selectCanPaste);
    const chatId = useAppSelector(selectChatId);

    const clearErrorMessage = useCallback(() => setErrorMessage(""), []);

    // move this
    const handleOpenFile = useCallback(async () => {
      if (!toolCall.function.arguments.path) return;
      await queryPathThenOpenFile({
        file_path: toolCall.function.arguments.path,
      });
    }, [toolCall.function.arguments.path, queryPathThenOpenFile]);

    const handleReplace = useCallback(
      (content: string) => {
        diffPasteBack(content, chatId, toolCall.id);
      },
      [chatId, diffPasteBack, toolCall.id],
    );

    const replaceContent = useMemo(() => {
      if (isCreateTextDocToolCall(toolCall))
        return toolCall.function.arguments.content;
      if (isUpdateTextDocToolCall(toolCall))
        return toolCall.function.arguments.replacement;
      if (isUpdateTextDocByLinesToolCall(toolCall))
        return toolCall.function.arguments.content;
      return null;
    }, [toolCall]);

    const handleApplyToolResult = useCallback(() => {
      requestDryRun({
        toolName: toolCall.function.name,
        toolArgs: toolCall.function.arguments,
      })
        .then((results) => {
          if (results.data) {
            sendToolCallToIde(toolCall, results.data, chatId);
          } else if (isRTKResponseErrorWithDetailMessage(results)) {
            setErrorMessage(results.error.data.detail);
          }
        })
        .catch((error: unknown) => {
          if (
            error &&
            typeof error === "object" &&
            "message" in error &&
            typeof error.message === "string"
          ) {
            setErrorMessage(error.message);
          } else {
            setErrorMessage("Error with patch: " + JSON.stringify(error));
          }
        });
    }, [chatId, requestDryRun, sendToolCallToIde, toolCall]);

    return (
      <Card
        size="1"
        variant="surface"
        mt="4"
        className={styles.textdoc__header}
        ref={ref}
      >
        <Flex gap="2" py="2" pl="2" justify="between">
          <TruncateLeft>
            <Link
              title="Open file"
              onClick={(event) => {
                event.preventDefault();
                void handleOpenFile();
              }}
            >
              {toolCall.function.arguments.path}
            </Link>
          </TruncateLeft>{" "}
          <div style={{ flexGrow: 1 }} />
          <Button
            size="1"
            onClick={handleApplyToolResult}
            disabled={dryRunResult.isLoading}
            // title={`Apply`}
            className={classNames(styles.apply_button)}
          >
            ➕ Diff
          </Button>
          {replaceContent && (
            <Button
              size="1"
              // this one can directly dismiss the tool confirmation.
              onClick={() => handleReplace(replaceContent)}
              disabled={dryRunResult.isLoading || !canPaste}
              // title="Replace the current selection in the ide."
            >
              ➕ Replace Selection
            </Button>
          )}
        </Flex>
        {errorMessage && (
          <ErrorCallout onClick={clearErrorMessage} timeout={5000}>
            {errorMessage}
          </ErrorCallout>
        )}
      </Card>
    );
  },
);
TextDocHeader.displayName = "TextDocHeader";

const CreateTextDoc: React.FC<{
  toolCall: CreateTextDocToolCall;
}> = ({ toolCall }) => {
  const handleCopy = useCopyToClipboard();
  const ref = useRef<HTMLDivElement>(null);
  const handleClose = useHideScroll(ref);

  const className = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return `language-${extension}`;
  }, [toolCall.function.arguments.path]);

  const lineCount = useMemo(
    () => toolCall.function.arguments.content.split("\n").length,
    [toolCall.function.arguments.content],
  );

  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} ref={ref} />

      <Reveal isRevealingCode defaultOpen={lineCount < 9} onClose={handleClose}>
        <MarkdownCodeBlock onCopyClick={handleCopy} className={className}>
          {toolCall.function.arguments.content}
        </MarkdownCodeBlock>
      </Reveal>
    </Box>
  );
};

const ReplaceTextDoc: React.FC<{
  toolCall: ReplaceTextDocToolCall;
}> = ({ toolCall }) => {
  const copyToClipBoard = useCopyToClipboard();
  const handleCopy = useCallback(() => {
    copyToClipBoard(toolCall.function.arguments.replacement);
  }, [copyToClipBoard, toolCall.function.arguments.replacement]);

  const ref = useRef<HTMLDivElement>(null);
  const handleClose = useHideScroll(ref);

  const className = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return `language-${extension}`;
  }, [toolCall.function.arguments.path]);

  const lineCount = useMemo(
    () => toolCall.function.arguments.replacement.split("\n").length,
    [toolCall.function.arguments.replacement],
  );

  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} ref={ref} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9} onClose={handleClose}>
        <MarkdownCodeBlock onCopyClick={handleCopy} className={className}>
          {toolCall.function.arguments.replacement}
        </MarkdownCodeBlock>
      </Reveal>
    </Box>
  );
};

const UpdateRegexTextDoc: React.FC<{
  toolCall: UpdateRegexTextDocToolCall;
}> = ({ toolCall }) => {
  const ref = useRef<HTMLDivElement>(null);
  const handleClose = useHideScroll(ref);
  const code = useMemo(() => {
    return (
      're.sub("' +
      toolCall.function.arguments.pattern +
      '", "' +
      toolCall.function.arguments.replacement +
      '", open("' +
      toolCall.function.arguments.path +
      '"))\n'
    );
  }, [
    toolCall.function.arguments.path,
    toolCall.function.arguments.pattern,
    toolCall.function.arguments.replacement,
  ]);

  const lineCount = useMemo(() => code.split("\n").length, [code]);

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} ref={ref} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9} onClose={handleClose}>
        <MarkdownCodeBlock className="language-py">{code}</MarkdownCodeBlock>
      </Reveal>
    </Box>
  );
};

const UpdateTextDoc: React.FC<{
  toolCall: UpdateTextDocToolCall;
}> = ({ toolCall }) => {
  const copyToClipBoard = useCopyToClipboard();
  const ref = useRef<HTMLDivElement>(null);
  const handleClose = useHideScroll(ref);
  const handleCopy = useCallback(() => {
    copyToClipBoard(toolCall.function.arguments.replacement);
  }, [copyToClipBoard, toolCall.function.arguments.replacement]);

  const className = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return `language-${extension}`;
  }, [toolCall.function.arguments.path]);

  const lineCount = useMemo(
    () => toolCall.function.arguments.replacement.split("\n").length,
    [toolCall.function.arguments.replacement],
  );

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} ref={ref} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9} onClose={handleClose}>
        <MarkdownCodeBlock onCopyClick={handleCopy} className={className}>
          {toolCall.function.arguments.replacement}
        </MarkdownCodeBlock>
      </Reveal>
    </Box>
  );
};

const UpdateTextDocByLines: React.FC<{
  toolCall: UpdateTextDocByLinesToolCall;
}> = ({ toolCall }) => {
  const copyToClipBoard = useCopyToClipboard();
  const ref = useRef<HTMLDivElement>(null);
  const handleClose = useHideScroll(ref);
  const handleCopy = useCallback(() => {
    copyToClipBoard(toolCall.function.arguments.content);
  }, [copyToClipBoard, toolCall.function.arguments.content]);

  const className = useMemo(() => {
    const extension = getFileExtension(toolCall.function.arguments.path);
    return `language-${extension}`;
  }, [toolCall.function.arguments.path]);

  const lineCount = useMemo(
    () => toolCall.function.arguments.content.split("\n").length,
    [toolCall.function.arguments.content],
  );

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} ref={ref} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9} onClose={handleClose}>
        <MarkdownCodeBlock onCopyClick={handleCopy} className={className}>
          {toolCall.function.arguments.content}
        </MarkdownCodeBlock>
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
