import React, { useCallback, useMemo, useState } from "react";
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
import { Box, Card, Flex, Button } from "@radix-ui/themes";
import { TruncateLeft } from "../Text";
import { Link } from "../Link";
import { useEventsBusForIDE } from "../../hooks/useEventBusForIDE";
import { Markdown } from "../Markdown";
import { filename } from "../../utils/filename";
import styles from "./Texdoc.module.css";
import { useCopyToClipboard } from "../../hooks/useCopyToClipboard";
import { Reveal } from "../Reveal";
import { useAppSelector } from "../../hooks";
import { selectCanPaste, selectChatId } from "../../features/Chat";
import { toolsApi } from "../../services/refact";
import { ErrorCallout } from "../Callout";
import { isRTKResponseErrorWithDetailMessage } from "../../utils";

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
  const { openFile, diffPasteBack, sendToolCallToIde } = useEventsBusForIDE();
  const [requestDryRun, dryRunResult] = toolsApi.useDryRunForEditToolMutation();
  const [errorMessage, setErrorMessage] = useState<string>("");
  const canPaste = useAppSelector(selectCanPaste);
  const chatId = useAppSelector(selectChatId);

  const clearErrorMessage = useCallback(() => setErrorMessage(""), []);

  // move this
  const handleOpenFile = useCallback(() => {
    if (!toolCall.function.arguments.path) return;
    openFile({ file_name: toolCall.function.arguments.path });
  }, [openFile, toolCall.function.arguments.path]);

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
        <div style={{ flexGrow: 1 }} />
        <Button
          size="1"
          onClick={handleApplyToolResult}
          disabled={dryRunResult.isLoading}
          title={`Apply`}
        >
          ➕ Apply
        </Button>
        {replaceContent && (
          <Button
            size="1"
            // this one can directly dismiss the tool confirmation.
            onClick={() => handleReplace(replaceContent)}
            disabled={dryRunResult.isLoading || !canPaste}
            title="Replace the current selection in the ide."
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

  const copyToClipBoard = useCopyToClipboard();
  const handleCopy = useCallback(() => {
    copyToClipBoard(toolCall.function.arguments.replacement);
  }, [copyToClipBoard, toolCall.function.arguments.replacement]);

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Reveal isRevealingCode defaultOpen={lineCount < 9}>
        <Markdown onCopyClick={handleCopy}>{code}</Markdown>
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
