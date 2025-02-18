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
import { Box, Button, Card, Flex } from "@radix-ui/themes";
import { TruncateLeft } from "../Text";
import { Link } from "../Link";
import { useEventsBusForIDE } from "../../hooks/useEventBusForIDE";
import { Markdown } from "../Markdown";
import { filename } from "../../utils/filename";
import styles from "./Texdoc.module.css";
import { createPatch } from "diff";
import classNames from "classnames";
import { useAppSelector } from "../../hooks";
import { selectCanPaste } from "../../features/Chat";
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
  const { openFile, diffPasteBack, sendToolEditToIde } = useEventsBusForIDE();
  const [requestDryRun, dryRunResult] = toolsApi.useDryRunForEditToolMutation();
  const [errorMessage, setErrorMessage] = useState<string>("");
  const canPaste = useAppSelector(selectCanPaste);

  const clearErrorMessage = useCallback(() => setErrorMessage(""), []);
  // move this
  const handleOpenFile = useCallback(() => {
    if (!toolCall.function.arguments.path) return;
    openFile({ file_name: toolCall.function.arguments.path });
  }, [openFile, toolCall.function.arguments.path]);

  const handleReplace = useCallback(
    (content: string) => {
      diffPasteBack(content);
    },
    [diffPasteBack],
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
          sendToolEditToIde(toolCall.function.arguments.path, results.data);
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
  }, [
    requestDryRun,
    sendToolEditToIde,
    toolCall.function.arguments,
    toolCall.function.name,
  ]);

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
  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Markdown>{code}</Markdown>
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
  return (
    // TODO: move this box up a bit, or make it generic
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Markdown>{code}</Markdown>
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

  return (
    <Box className={styles.textdoc}>
      <TextDocHeader toolCall={toolCall} />
      <Markdown>{code}</Markdown>
    </Box>
  );
};

const UpdateTextDoc: React.FC<{
  toolCall: UpdateTextDocToolCall;
}> = ({ toolCall }) => {
  const diff = useMemo(() => {
    const patch = createPatch(
      toolCall.function.arguments.path,
      toolCall.function.arguments.old_str,
      toolCall.function.arguments.replacement,
    );

    return "```diff\n" + patch + "\n```";
  }, [
    toolCall.function.arguments.replacement,
    toolCall.function.arguments.old_str,
    toolCall.function.arguments.path,
  ]);
  // TODO: don't use markdown for this, it's two bright
  return (
    <Box className={classNames(styles.textdoc, styles.textdoc__update)}>
      <TextDocHeader toolCall={toolCall} />
      <Box className={classNames(styles.textdoc__diffbox)}>
        <Markdown useInlineStyles={false}>{diff}</Markdown>
      </Box>
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
