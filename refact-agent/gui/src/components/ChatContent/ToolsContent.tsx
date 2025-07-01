import React, { forwardRef, useCallback, useMemo, useRef } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import {
  Container,
  Flex,
  Text,
  Box,
  Spinner,
  Card,
  Separator,
} from "@radix-ui/themes";
import {
  isMultiModalToolMessage,
  MultiModalToolMessage,
  // MultiModalToolResult,
  // knowledgeApi,
  // MultiModalToolResult,
  ToolCall,
  type ToolMessage,
  // ToolResult,
  ToolUsage,
} from "../../services/refact";
import styles from "./ChatContent.module.css";
import { CommandMarkdown } from "../Command";
import { Chevron } from "../Collapsible";
import { Reveal } from "../Reveal";
import { useAppSelector, useHideScroll } from "../../hooks";
import {
  selectManyToolMessagesByIds,
  selectManyDiffMessageByIds,
  selectToolMessageById,
} from "../../features/ThreadMessages";
import {
  selectIsStreaming,
  selectIsWaiting,
} from "../../features/ThreadMessages";
import { ScrollArea } from "../ScrollArea";
import { takeWhile } from "../../utils";
import { DialogImage } from "../DialogImage";
import { RootState } from "../../app/store";
import { selectFeatures } from "../../features/Config/configSlice";
import { isRawTextDocToolCall } from "../Tools/types";
import { TextDocTool } from "../Tools/Textdoc";
import { MarkdownCodeBlock } from "../Markdown/CodeBlock";
import classNames from "classnames";
import resultStyle from "react-syntax-highlighter/dist/esm/styles/hljs/arta";
import { FadedButton } from "../Buttons";
import { AnimatedText } from "../Text";
type ResultProps = {
  children: string;
  isInsideScrollArea?: boolean;
  onClose?: () => void;
};

const Result: React.FC<ResultProps> = ({ children, onClose }) => {
  const lines = children.split("\n");
  return (
    <Reveal defaultOpen={lines.length < 9} isRevealingCode onClose={onClose}>
      <MarkdownCodeBlock
        className={classNames(styles.tool_result)}
        style={resultStyle}
      >
        {children}
      </MarkdownCodeBlock>
    </Reveal>
  );
};

function toolCallArgsToString(toolCallArgs: string) {
  try {
    const json = JSON.parse(toolCallArgs) as unknown as Parameters<
      typeof Object.entries
    >;
    if (Array.isArray(json)) {
      return json.join(", ");
    }
    return Object.entries(json)
      .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
      .join(", ");
  } catch {
    return toolCallArgs;
  }
}

// TODO: Sort of duplicated
const ToolMessage: React.FC<{
  toolCall: ToolCall;
  onClose: () => void;
}> = ({ toolCall, onClose }) => {
  const name = toolCall.function.name ?? "";
  const maybeResult = useAppSelector((state) =>
    selectToolMessageById(state, toolCall.id),
  );

  const argsString = React.useMemo(() => {
    return toolCallArgsToString(toolCall.function.arguments);
  }, [toolCall.function.arguments]);

  if (maybeResult && isMultiModalToolMessage(maybeResult)) {
    // TODO: handle this
    return null;
  }

  const functionCalled = "```python\n" + name + "(" + argsString + ")\n```";

  return (
    <Flex direction="column">
      <ScrollArea scrollbars="horizontal" style={{ width: "100%" }}>
        <Box>
          <CommandMarkdown isInsideScrollArea>{functionCalled}</CommandMarkdown>
        </Box>
      </ScrollArea>
      {maybeResult?.ftm_content && (
        <Result isInsideScrollArea onClose={onClose}>
          {maybeResult.ftm_content}
        </Result>
      )}
    </Flex>
  );
};

const ToolUsageDisplay: React.FC<{
  functionName: string;
  amountOfCalls: number;
}> = ({ functionName, amountOfCalls }) => {
  return (
    <>
      {functionName}
      {amountOfCalls > 1 ? ` (${amountOfCalls})` : ""}
    </>
  );
};

// Use this for a single tool results
export const SingleModelToolContent: React.FC<{
  toolCalls: ToolCall[];
}> = ({ toolCalls }) => {
  const [open, setOpen] = React.useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const handleHide = useHideScroll(ref);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const toolCallsId = useMemo(() => {
    const ids = toolCalls.reduce<string[]>((acc, toolCall) => {
      if (typeof toolCall.id === "string") return [...acc, toolCall.id];
      return acc;
    }, []);

    return ids;
  }, [toolCalls]);

  const results = useAppSelector((state) =>
    selectManyToolMessagesByIds(state, toolCallsId),
  );

  const diffs = useAppSelector((state) =>
    selectManyDiffMessageByIds(state, toolCallsId),
  );
  const allResolved = useMemo(() => {
    return results.length + diffs.length === toolCallsId.length;
  }, [diffs.length, results.length, toolCallsId.length]);

  const busy = useMemo(() => {
    if (allResolved) return false;
    return isStreaming || isWaiting;
  }, [allResolved, isStreaming, isWaiting]);

  const handleClose = useCallback(() => {
    handleHide();
    setOpen(false);
  }, [handleHide]);

  if (toolCalls.length === 0) return null;

  const toolNames = toolCalls.reduce<string[]>((acc, toolCall) => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (toolCall === null) {
      // eslint-disable-next-line no-console
      console.error("toolCall is null");
      return acc;
    }
    if (!toolCall.function.name) return acc;
    if (acc.includes(toolCall.function.name)) return acc;
    return [...acc, toolCall.function.name];
  }, []);

  /*
    Calculates the usage amount of each tool by mapping over the unique tool names
    and counting how many times each tool has been called in the toolCalls array.
  */
  const toolUsageAmount = toolNames.map<ToolUsage>((toolName) => {
    return {
      functionName: toolName,
      amountOfCalls: toolCalls.filter(
        (toolCall) => toolCall.function.name === toolName,
      ).length,
    };
  });

  const subchat: string | undefined = toolCalls
    .map((toolCall) => toolCall.subchat)
    .filter((x) => x)[0];
  const attachedFiles = toolCalls
    .map((toolCall) => toolCall.attached_files)
    .filter((x) => x)
    .flat();
  const shownAttachedFiles = attachedFiles.slice(-4);
  const hiddenFiles = attachedFiles.length - 4;

  // Use this for single tool result
  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <ToolUsageSummary
            ref={ref}
            toolUsageAmount={toolUsageAmount}
            hiddenFiles={hiddenFiles}
            shownAttachedFiles={shownAttachedFiles}
            subchat={subchat}
            open={open}
            onClick={() => setOpen((prev) => !prev)}
            waiting={busy}
          />
        </Collapsible.Trigger>
        <Collapsible.Content>
          {toolCalls.map((toolCall) => {
            // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
            if (toolCall === null) {
              // eslint-disable-next-line no-console
              console.error("toolCall is null");
              return;
            }
            const key = `${toolCall.id}-${toolCall.index}`;
            return (
              <Box key={key} py="2">
                <ToolMessage toolCall={toolCall} onClose={handleClose} />
              </Box>
            );
          })}
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

export type ToolContentProps = {
  toolCalls: ToolCall[];
};

export const ToolContent: React.FC<ToolContentProps> = ({ toolCalls }) => {
  const features = useAppSelector(selectFeatures);
  const ids = toolCalls.reduce<string[]>((acc, cur) => {
    if (cur.id) return [...acc, cur.id];
    return acc;
  }, []);
  // Chate this selector to use thread message list
  const allToolResults = useAppSelector((state) =>
    selectManyToolMessagesByIds(state, ids),
  );

  return processToolCalls(toolCalls, allToolResults, features);
};

function processToolCalls(
  toolCalls: ToolCall[],
  toolResults: ToolMessage[],
  features: RootState["config"]["features"] = {},
  processed: React.ReactNode[] = [],
) {
  if (toolCalls.length === 0) return processed;
  const [head, ...tail] = toolCalls;
  const result = toolResults.find((result) => result.ftm_call_id === head.id);

  // TODO: handle knowledge differently.
  // memories are split in content with 🗃️019957b6ff

  if (result && head.function.name === "knowledge") {
    const elem = (
      <Knowledge key={`knowledge-tool-${processed.length}`} toolCall={head} />
    );
    return processToolCalls(tail, toolResults, features, [...processed, elem]);
  }

  if (isRawTextDocToolCall(head)) {
    const elem = (
      <TextDocTool
        key={`textdoc-tool-${head.function.name}-${processed.length}`}
        toolCall={head}
        // TODO: failed tools
        // toolFailed={result?.tool_failed}
      />
    );
    return processToolCalls(tail, toolResults, features, [...processed, elem]);
  }

  // TODO: skip multi modal for now
  if (result && isMultiModalToolMessage(result)) {
    const restInTail = takeWhile(tail, (toolCall) => {
      const nextResult = toolResults.find(
        (res) => res.ftm_call_id === toolCall.id,
      );
      return nextResult !== undefined && isMultiModalToolMessage(nextResult);
    });

    const nextTail = tail.slice(restInTail.length);
    const multiModalToolCalls = [head, ...restInTail];
    const ids = multiModalToolCalls.map((d) => d.id);
    const multiModalToolResults: MultiModalToolMessage[] = toolResults
      // .filter(isMultiModalToolResult)
      .filter(isMultiModalToolMessage)
      .filter((toolResult) => ids.includes(toolResult.ftm_call_id));

    const elem = (
      <MultiModalToolContent
        key={`multi-model-tool-content-${processed.length}`}
        toolCalls={multiModalToolCalls}
        toolResults={multiModalToolResults}
      />
    );
    return processToolCalls(nextTail, toolResults, features, [
      ...processed,
      elem,
    ]);
  }

  const restInTail = takeWhile(tail, (toolCall) => {
    const item = toolResults.find(
      (result) => result.ftm_call_id === toolCall.id,
    );
    return item === undefined; // || !isMultiModalToolResult(item);
  });
  const nextTail = tail.slice(restInTail.length);

  const elem = (
    <SingleModelToolContent
      key={`single-model-tool-call-${processed.length}`}
      toolCalls={[head, ...restInTail]}
    />
  );
  return processToolCalls(nextTail, toolResults, features, [
    ...processed,
    elem,
  ]);
}

const MultiModalToolContent: React.FC<{
  toolCalls: ToolCall[];
  toolResults: MultiModalToolMessage[];
}> = ({ toolCalls, toolResults }) => {
  const [open, setOpen] = React.useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const handleHide = useHideScroll(ref);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const ids = useMemo(() => {
    return toolCalls.reduce<string[]>((acc, cur) => {
      if (typeof cur === "string") return [...acc, cur];
      return acc;
    }, []);
  }, [toolCalls]);

  const diffs = useAppSelector((state) =>
    selectManyDiffMessageByIds(state, ids),
  );

  const handleClose = useCallback(() => {
    handleHide();
    setOpen(false);
  }, [handleHide]);
  // const content = toolResults.map((toolResult) => toolResult.content);

  const hasImages = toolResults.some((toolResult) =>
    toolResult.ftm_content.some((content) =>
      content.m_type.startsWith("image/"),
    ),
  );

  // TOOD: duplicated
  const toolNames = toolCalls.reduce<string[]>((acc, toolCall) => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (toolCall === null) {
      // eslint-disable-next-line no-console
      console.error("toolCall is null");
      return acc;
    }
    if (!toolCall.function.name) return acc;
    if (acc.includes(toolCall.function.name)) return acc;
    return [...acc, toolCall.function.name];
  }, []);

  // TODO: duplicated
  const toolUsageAmount = toolNames.map<ToolUsage>((toolName) => {
    return {
      functionName: toolName,
      amountOfCalls: toolCalls.filter(
        (toolCall) => toolCall.function.name === toolName,
      ).length,
    };
  });

  const hasResults = useMemo(() => {
    // TODO: diffs
    const diffIds = diffs.map((diff) => diff.ftm_call_id);
    const toolIds = toolResults.map((d) => d.ftm_call_id);
    const resultIds = [...diffIds, ...toolIds];
    return toolCalls.every(
      (toolCall) => toolCall.id && resultIds.includes(toolCall.id),
    );
  }, [toolCalls, toolResults, diffs]);

  const busy = useMemo(() => {
    if (hasResults) return false;
    return isStreaming || isWaiting;
  }, [hasResults, isStreaming, isWaiting]);

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <ToolUsageSummary
            toolUsageAmount={toolUsageAmount}
            open={open}
            onClick={() => setOpen((prev) => !prev)}
            ref={ref}
            waiting={busy}
          />
        </Collapsible.Trigger>
        <Collapsible.Content>
          {/** TODO: tool call name and text result */}
          <Box py="2">
            {toolCalls.map((toolCall, i) => {
              const result = toolResults.find(
                (toolResult) => toolResult.ftm_call_id === toolCall.id,
              );
              if (!result) return null;

              const texts = result.ftm_content
                .filter((content) => content.m_type === "text")
                .map((result) => result.m_content)
                .join("\n");

              const name = toolCall.function.name ?? "";
              const argsString = toolCallArgsToString(
                toolCall.function.arguments,
              );

              const functionCalled =
                "```python\n" + name + "(" + argsString + ")\n```";

              // TODO: sort of duplicated
              return (
                <Flex
                  direction="column"
                  key={`tool-call-command-${toolCall.id}-${i}`}
                  py="2"
                  ref={ref}
                >
                  <ScrollArea scrollbars="horizontal" style={{ width: "100%" }}>
                    <Box>
                      <CommandMarkdown isInsideScrollArea>
                        {functionCalled}
                      </CommandMarkdown>
                    </Box>
                  </ScrollArea>
                  <Box>
                    <Result onClose={handleClose}>{texts}</Result>
                  </Box>
                </Flex>
              );
            })}
          </Box>
        </Collapsible.Content>
      </Collapsible.Root>
      {hasImages && (
        <Flex py="2" gap="2" wrap="wrap">
          {toolCalls.map((toolCall, index) => {
            const toolResult = toolResults.find(
              (toolResult) => toolResult.ftm_call_id === toolCall.id,
            );
            if (!toolResult) return null;

            const images = toolResult.ftm_content.filter((content) =>
              content.m_type.startsWith("image/"),
            );
            if (images.length === 0) return null;

            return images.map((image, idx) => {
              const dataUrl = `data:${image.m_type};base64,${image.m_content}`;
              const key = `tool-image-${toolResult.ftm_call_id}-${index}-${idx}`;
              return (
                <DialogImage key={key} size="8" src={dataUrl} fallback="" />
              );
            });
          })}
        </Flex>
      )}
    </Container>
  );
};

type ToolUsageSummaryProps = {
  toolUsageAmount: ToolUsage[];
  hiddenFiles?: number;
  shownAttachedFiles?: (string | undefined)[];
  subchat?: string;
  open: boolean;
  onClick?: () => void;
  waiting: boolean;
};

const ToolUsageSummary = forwardRef<HTMLDivElement, ToolUsageSummaryProps>(
  (
    {
      toolUsageAmount,
      hiddenFiles,
      shownAttachedFiles,
      subchat,
      open,
      onClick,
      waiting,
    },
    ref,
  ) => {
    return (
      <AnimatedText as="div" weight="light" size="1" animating={waiting}>
        <Flex gap="2" align="end" onClick={onClick} ref={ref} my="2">
          <Flex
            gap="1"
            align="start"
            direction="column"
            style={{ cursor: "pointer" }}
          >
            <Flex gap="2" align="center" justify="center">
              {waiting ? <Spinner /> : "🔨"} {/* 🔨{" "} */}
              {toolUsageAmount.map(({ functionName, amountOfCalls }, index) => (
                <span key={functionName}>
                  <ToolUsageDisplay
                    functionName={functionName}
                    amountOfCalls={amountOfCalls}
                  />
                  {index === toolUsageAmount.length - 1 ? "" : ", "}
                </span>
              ))}
            </Flex>

            {hiddenFiles && hiddenFiles > 0 && (
              <Text weight="light" size="1" ml="4">
                {`🔎 <${hiddenFiles} files hidden>`}
              </Text>
            )}
            {shownAttachedFiles?.map((file, index) => {
              if (!file) return null;

              return (
                <Text weight="light" size="1" key={index} ml="4">
                  🔎 {file}
                </Text>
              );
            })}
            {subchat && (
              <Flex ml="4">
                {waiting && <Spinner />}
                <Text weight="light" size="1" ml="4px">
                  {subchat}
                </Text>
              </Flex>
            )}
          </Flex>
          <Chevron open={open} />
        </Flex>
      </AnimatedText>
    );
  },
);
ToolUsageSummary.displayName = "ToolUsageSummary";

// TODO: make this look nicer.
const Knowledge: React.FC<{ toolCall: ToolCall }> = ({ toolCall }) => {
  const [open, setOpen] = React.useState(false);
  const ref = useRef(null);
  const scrollOnHide = useHideScroll(ref);

  const handleHide = useCallback(() => {
    setOpen(false);
    scrollOnHide();
  }, [scrollOnHide]);

  const maybeResult = useAppSelector((state) =>
    selectToolMessageById(state, toolCall.id),
  );

  const argsString = React.useMemo(() => {
    return toolCallArgsToString(toolCall.function.arguments);
  }, [toolCall.function.arguments]);

  const memories = useMemo(() => {
    if (typeof maybeResult?.ftm_content !== "string") return [];
    return splitMemories(maybeResult.ftm_content);
  }, [maybeResult?.ftm_content]);

  const functionCalled = "```python\n" + name + "(" + argsString + ")\n```";

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex
            gap="2"
            align="end"
            onClick={() => setOpen((prev) => !prev)}
            ref={ref}
          >
            <Flex
              gap="1"
              align="start"
              direction="column"
              style={{ cursor: "pointer" }}
            >
              <Text weight="light" size="1">
                📚 Knowledge
              </Text>
            </Flex>
            <Chevron open={open} />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <Flex direction="column" pt="4">
            <ScrollArea scrollbars="horizontal" style={{ width: "100%" }}>
              <Box>
                <CommandMarkdown isInsideScrollArea>
                  {functionCalled}
                </CommandMarkdown>
              </Box>
            </ScrollArea>
            <Flex gap="4" direction="column" py="4">
              {memories.map((memory) => {
                return (
                  <Memory
                    key={memory.memid}
                    id={memory.memid}
                    content={memory.content}
                  />
                );
              })}
            </Flex>
            <FadedButton color="gray" onClick={handleHide} mx="2">
              Hide Memories
            </FadedButton>
          </Flex>
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

const Memory: React.FC<{ id: string; content: string }> = ({ id, content }) => {
  return (
    <Card>
      <Flex direction="column" gap="2">
        <Flex justify="between" align="center">
          <Text size="1" weight="light">
            Memory: {id}
          </Text>
        </Flex>
        <Separator size="4" />
        <Text size="2">{content}</Text>
      </Flex>
    </Card>
  );
};

function splitMemories(text: string): { memid: string; content: string }[] {
  // Split by 🗃️ and filter out empty strings
  const parts = text.split("🗃️").filter((part) => part.trim());

  return parts.map((part) => {
    const newlineIndex = part.indexOf("\n");
    const memid = part.substring(0, newlineIndex);
    const content = part.substring(newlineIndex + 1);

    return {
      memid,
      content,
    };
  });
}
