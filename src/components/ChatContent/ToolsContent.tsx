import React from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Container, Flex, Text, Box, Spinner } from "@radix-ui/themes";
import { ToolCall, ToolResult, ToolUsage } from "../../services/refact";
import styles from "./ChatContent.module.css";
import { CommandMarkdown, ResultMarkdown } from "../Command";
import { Chevron } from "../Collapsible";
import { Reveal } from "../Reveal";
import { useAppSelector } from "../../hooks";
import { selectToolResultById } from "../../features/Chat/Thread/selectors";
import { ScrollArea } from "../ScrollArea";

type ResultProps = {
  children: string;
  isInsideScrollArea?: boolean;
  hasImages: boolean;
};

const Result: React.FC<ResultProps> = ({
  children,
  isInsideScrollArea = false,
  hasImages,
}) => {
  const lines = children.split("\n");
  return (
    <Reveal defaultOpen={!hasImages && lines.length < 9}>
      <ResultMarkdown
        className={styles.tool_result}
        isInsideScrollArea={isInsideScrollArea}
      >
        {children}
      </ResultMarkdown>
    </Reveal>
  );
};

function resultToMarkdown(result?: ToolResult): string {
  if (!result) return "";
  if (!result.content) return "";

  if (typeof result.content === "string") {
    const escapedBackticks = result.content.replace(/`+/g, (match) => {
      if (match === "```") return match;
      return "\\" + "`";
    });

    return "```\n" + escapedBackticks + "\n```";
  }

  const images = result.content
    .filter((image) => image.m_type.startsWith("image/"))
    .map((image) => {
      const base64url = `data:${image.m_type};base64,${image.m_content}`;
      return `![](${base64url})`;
    });
  return images.join("\n");
}

const ToolMessage: React.FC<{
  toolCall: ToolCall;
}> = ({ toolCall }) => {
  const name = toolCall.function.name ?? "";

  const maybeResult = useAppSelector((state) =>
    selectToolResultById(state, toolCall.id),
  );

  const results = resultToMarkdown(maybeResult);
  const hasImages =
    Array.isArray(maybeResult?.content) &&
    maybeResult.content.some((image) => image.m_type.startsWith("image/"));

  const argsString = React.useMemo(() => {
    try {
      const json = JSON.parse(
        toolCall.function.arguments,
      ) as unknown as Parameters<typeof Object.entries>;
      if (Array.isArray(json)) {
        return json.join(", ");
      }
      return Object.entries(json)
        .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
        .join(", ");
    } catch {
      return toolCall.function.arguments;
    }
  }, [toolCall.function.arguments]);

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
          <Result hasImages={hasImages} isInsideScrollArea>
            {results}
          </Result>
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

export const ToolContent: React.FC<{
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

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="end">
            <Flex gap="1" align="start" direction="column">
              <Text weight="light" size="1">
                🔨{" "}
                {toolUsageAmount.map(
                  ({ functionName, amountOfCalls }, index) => (
                    <span key={functionName}>
                      <ToolUsageDisplay
                        functionName={functionName}
                        amountOfCalls={amountOfCalls}
                      />
                      {index === toolUsageAmount.length - 1 ? "" : ", "}
                    </span>
                  ),
                )}
              </Text>
              {hiddenFiles > 0 && (
                <Text weight="light" size="1" ml="4">
                  {`🔎 <${hiddenFiles} files hidden>`}
                </Text>
              )}
              {shownAttachedFiles.map((file, index) => (
                <Text weight="light" size="1" key={index} ml="4">
                  🔎 {file}
                </Text>
              ))}
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
