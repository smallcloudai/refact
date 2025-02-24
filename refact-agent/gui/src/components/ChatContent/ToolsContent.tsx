import React, { useCallback, useMemo } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import {
  Container,
  Flex,
  Text,
  Box,
  Spinner,
  IconButton,
  Card,
  Separator,
} from "@radix-ui/themes";
import {
  isMultiModalToolResult,
  knowledgeApi,
  MultiModalToolResult,
  ToolCall,
  ToolResult,
  ToolUsage,
} from "../../services/refact";
import styles from "./ChatContent.module.css";
import { CommandMarkdown, ResultMarkdown } from "../Command";
import { Chevron } from "../Collapsible";
import { Reveal } from "../Reveal";
import { useAppSelector } from "../../hooks";
import {
  selectManyToolResultsByIds,
  selectToolResultById,
} from "../../features/Chat/Thread/selectors";
import { ScrollArea } from "../ScrollArea";
import { takeWhile, fenceBackTicks } from "../../utils";
import { DialogImage } from "../DialogImage";
import { CheckIcon, Cross2Icon } from "@radix-ui/react-icons";
import { RootState } from "../../app/store";
import { selectFeatures } from "../../features/Config/configSlice";
import { isRawTextDocToolCall } from "../Tools/types";
import { TextDocTool } from "../Tools/Textdoc";

type ResultProps = {
  children: string;
  isInsideScrollArea?: boolean;
};

const Result: React.FC<ResultProps> = ({
  children,
  isInsideScrollArea = false,
}) => {
  const lines = children.split("\n");
  return (
    <Reveal defaultOpen={lines.length < 9} isRevealingCode>
      <ResultMarkdown
        className={styles.tool_result}
        isInsideScrollArea={isInsideScrollArea}
      >
        {children}
      </ResultMarkdown>
    </Reveal>
  );
};

function resultToMarkdown(content?: string): string {
  if (!content) return "";

  const escapedBackticks = fenceBackTicks(content);

  return "```\n" + escapedBackticks + "\n```";
}

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
}> = ({ toolCall }) => {
  const name = toolCall.function.name ?? "";

  // ToolResult could be multi modal
  // hoist this up
  const maybeResult = useAppSelector((state) =>
    selectToolResultById(state, toolCall.id),
  );

  const argsString = React.useMemo(() => {
    return toolCallArgsToString(toolCall.function.arguments);
  }, [toolCall.function.arguments]);

  if (maybeResult && isMultiModalToolResult(maybeResult)) {
    // TODO: handle this
    return null;
  }

  const results = resultToMarkdown(maybeResult?.content);

  const functionCalled = "```python\n" + name + "(" + argsString + ")\n```";

  return (
    <Flex direction="column">
      <ScrollArea scrollbars="horizontal" style={{ width: "100%" }}>
        <Box>
          <CommandMarkdown isInsideScrollArea>{functionCalled}</CommandMarkdown>
        </Box>
      </ScrollArea>
      <ScrollArea scrollbars="horizontal" style={{ width: "100%" }} asChild>
        <Box>
          <Result isInsideScrollArea>{results}</Result>
        </Box>
      </ScrollArea>
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
            toolUsageAmount={toolUsageAmount}
            hiddenFiles={hiddenFiles}
            shownAttachedFiles={shownAttachedFiles}
            subchat={subchat}
            open={open}
            onClick={() => setOpen((prev) => !prev)}
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
            if (toolCall.id === undefined) return;
            const key = `${toolCall.id}-${toolCall.index}`;
            return (
              <Box key={key} py="2">
                <ToolMessage toolCall={toolCall} />
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
    if (cur.id !== undefined) return [...acc, cur.id];
    return acc;
  }, []);
  const allToolResults = useAppSelector(selectManyToolResultsByIds(ids));

  return processToolCalls(toolCalls, allToolResults, features);
};

function processToolCalls(
  toolCalls: ToolCall[],
  toolResults: ToolResult[],
  features: RootState["config"]["features"] = {},
  processed: React.ReactNode[] = [],
) {
  if (toolCalls.length === 0) return processed;
  const [head, ...tail] = toolCalls;
  const result = toolResults.find((result) => result.tool_call_id === head.id);

  // TODO: handle knowledge differently.
  // memories are split in content with 🗃️019957b6ff

  if (features.knowledge && result && head.function.name === "knowledge") {
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
      />
    );
    return processToolCalls(tail, toolResults, features, [...processed, elem]);
  }

  if (result && isMultiModalToolResult(result)) {
    const restInTail = takeWhile(tail, (toolCall) => {
      const nextResult = toolResults.find(
        (res) => res.tool_call_id === toolCall.id,
      );
      return nextResult !== undefined && isMultiModalToolResult(nextResult);
    });

    const nextTail = tail.slice(restInTail.length);
    const multiModalToolCalls = [head, ...restInTail];
    const ids = multiModalToolCalls.map((d) => d.id);
    const multiModalToolResults: MultiModalToolResult[] = toolResults
      .filter(isMultiModalToolResult)
      .filter((toolResult) => ids.includes(toolResult.tool_call_id));

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
      (result) => result.tool_call_id === toolCall.id,
    );
    return item === undefined || !isMultiModalToolResult(item);
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
  toolResults: MultiModalToolResult[];
}> = ({ toolCalls, toolResults }) => {
  const [open, setOpen] = React.useState(false);

  // const content = toolResults.map((toolResult) => toolResult.content);

  const hasImages = toolResults.some((toolResult) =>
    toolResult.content.some((content) => content.m_type.startsWith("image/")),
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

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <ToolUsageSummary
            toolUsageAmount={toolUsageAmount}
            open={open}
            onClick={() => setOpen((prev) => !prev)}
          />
        </Collapsible.Trigger>
        <Collapsible.Content>
          {/** TODO: tool call name and text result */}
          <Box py="2">
            {toolCalls.map((toolCall, i) => {
              const result = toolResults.find(
                (toolResult) => toolResult.tool_call_id === toolCall.id,
              );
              if (!result) return null;

              const texts = result.content
                .filter((content) => content.m_type === "text")
                .map((result) => result.m_content)
                .join("\n");

              const name = toolCall.function.name ?? "";
              const argsString = toolCallArgsToString(
                toolCall.function.arguments,
              );

              const md = resultToMarkdown(texts);

              const functionCalled =
                "```python\n" + name + "(" + argsString + ")\n```";

              // TODO: sort of duplicated
              return (
                <Flex
                  direction="column"
                  key={`tool-call-command-${toolCall.id}-${i}`}
                  py="2"
                >
                  <ScrollArea scrollbars="horizontal" style={{ width: "100%" }}>
                    <Box>
                      <CommandMarkdown isInsideScrollArea>
                        {functionCalled}
                      </CommandMarkdown>
                    </Box>
                  </ScrollArea>
                  <ScrollArea
                    scrollbars="horizontal"
                    style={{ width: "100%" }}
                    asChild
                  >
                    <Box>
                      <Result>{md}</Result>
                    </Box>
                  </ScrollArea>
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
              (toolResult) => toolResult.tool_call_id === toolCall.id,
            );
            if (!toolResult) return null;

            const images = toolResult.content.filter((content) =>
              content.m_type.startsWith("image/"),
            );
            if (images.length === 0) return null;

            return images.map((image, idx) => {
              const dataUrl = `data:${image.m_type};base64,${image.m_content}`;
              const key = `tool-image-${toolResult.tool_call_id}-${index}-${idx}`;
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

const ToolUsageSummary: React.FC<{
  toolUsageAmount: ToolUsage[];
  hiddenFiles?: number;
  shownAttachedFiles?: (string | undefined)[];
  subchat?: string;
  open: boolean;
  onClick?: () => void;
}> = ({
  toolUsageAmount,
  hiddenFiles,
  shownAttachedFiles,
  subchat,
  open,
  onClick,
}) => {
  return (
    <Flex gap="2" align="end" onClick={onClick}>
      <Flex
        gap="1"
        align="start"
        direction="column"
        style={{ cursor: "pointer" }}
      >
        <Text weight="light" size="1">
          🔨{" "}
          {toolUsageAmount.map(({ functionName, amountOfCalls }, index) => (
            <span key={functionName}>
              <ToolUsageDisplay
                functionName={functionName}
                amountOfCalls={amountOfCalls}
              />
              {index === toolUsageAmount.length - 1 ? "" : ", "}
            </span>
          ))}
        </Text>
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
            <Spinner />
            <Text weight="light" size="1" ml="4px">
              {subchat}
            </Text>
          </Flex>
        )}
      </Flex>
      <Chevron open={open} />
    </Flex>
  );
};

// TODO: make this look nicer.
const Knowledge: React.FC<{ toolCall: ToolCall }> = ({ toolCall }) => {
  const [open, setOpen] = React.useState(false);

  // TODO: add voting actions
  const maybeResult = useAppSelector((state) =>
    selectToolResultById(state, toolCall.id),
  );

  const argsString = React.useMemo(() => {
    return toolCallArgsToString(toolCall.function.arguments);
  }, [toolCall.function.arguments]);

  const memories = useMemo(() => {
    if (typeof maybeResult?.content !== "string") return [];
    return splitMemories(maybeResult.content);
  }, [maybeResult?.content]);

  const functionCalled = "```python\n" + name + "(" + argsString + ")\n```";

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="end" onClick={() => setOpen((prev) => !prev)}>
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
          </Flex>
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

const Memory: React.FC<{ id: string; content: string }> = ({ id, content }) => {
  const [updateUsage, status] = knowledgeApi.useUpdateMemoryUsageMutation();
  // correct and relevant,
  const handleGood = useCallback(() => {
    void updateUsage({ memid: id, correct: 1, relevant: 1 });
  }, [id, updateUsage]);

  // TODO: not correct but relevant, and incorrect but relevant
  const handleBad = useCallback(() => {
    void updateUsage({ memid: id, correct: -1, relevant: -1 });
  }, [id, updateUsage]);

  return (
    <Card>
      <Flex direction="column" gap="2">
        <Flex justify="between" align="center">
          <Text size="1" weight="light">
            Memory: {id}
          </Text>
          <Flex gap="2" align="center">
            <IconButton
              size="1"
              title="Bad"
              onClick={handleBad}
              disabled={status.isLoading}
              variant="outline"
              color="tomato"
            >
              <Cross2Icon />{" "}
            </IconButton>
            <IconButton
              size="1"
              title="Good"
              onClick={handleGood}
              disabled={status.isLoading}
              variant="outline"
              color="grass"
            >
              <CheckIcon />
            </IconButton>
          </Flex>
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
