import React from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Container, Flex, Text, Box } from "@radix-ui/themes";
import { ToolCall, ToolResult } from "../../events";
import styles from "./ChatContent.module.css";
import { CommandMarkdown, ResultMarkdown } from "../Command";
import { Chevron } from "../Collapsible";
import { Reveal } from "../Reveal";

const Result: React.FC<{ children: string }> = ({ children }) => {
  const lines = children.split("\n");
  return (
    <Reveal defaultOpen={lines.length < 9}>
      <ResultMarkdown className={styles.tool_result}>{children}</ResultMarkdown>
    </Reveal>
  );
};

const ToolMessage: React.FC<{
  toolCall: ToolCall;
  result?: ToolResult;
}> = ({ toolCall, result }) => {
  const results = result?.content ?? "";
  const name = toolCall.function.name ?? "";

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

  const escapedBackticks = results.replace(/`+/g, (match) => {
    if (match === "```") return match;
    return "\\" + "`";
  });

  return (
    <Flex direction="column">
      <CommandMarkdown>{functionCalled}</CommandMarkdown>
      <Result>{"```\n" + escapedBackticks + "\b```"}</Result>
    </Flex>
  );
};

export const ToolContent: React.FC<{
  toolCalls: ToolCall[];
  results: Record<string, ToolResult>;
}> = ({ toolCalls, results }) => {
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

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="center">
            <Text weight="light" size="1">
              🔨 {toolNames.join(", ")} ({toolCalls.length})
            </Text>
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
            const result = results[toolCall.id];
            const key = `${toolCall.id}-${toolCall.index}`;
            return (
              <Box key={key} py="2">
                <ToolMessage toolCall={toolCall} result={result} />
              </Box>
            );
          })}
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};
