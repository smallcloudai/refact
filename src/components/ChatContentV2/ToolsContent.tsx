import React from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Container, Flex, Text, Box } from "@radix-ui/themes";
import { ToolCall, ToolResult } from "../../events";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./ChatContent.module.css";
import { Markdown } from "../CommandLine/Markdown";

const ToolMessage: React.FC<{
  toolCall: ToolCall;
  result?: ToolResult;
}> = ({ toolCall, result }) => {
  // TODO: different component for each tool call name?
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

  if (!result?.content) {
    return <Markdown>{functionCalled}</Markdown>;
  }

  return <Markdown>{functionCalled + "\n" + result.content}</Markdown>;
};

export const ToolContent: React.FC<{
  toolCalls: ToolCall[];
  results: Record<string, ToolResult>;
}> = ({ toolCalls, results }) => {
  const [open, setOpen] = React.useState(false);
  const resultIds = Object.keys(results);
  const allResolved = toolCalls.every(
    (toolCall) => toolCall.id && resultIds.includes(toolCall.id),
  );

  if (toolCalls.length === 0) return null;

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="center">
            <Text weight="light" size="1">
              {allResolved ? "Used" : "Using"} {toolCalls.length}{" "}
              {toolCalls.length > 1 ? "tools" : "tool"}
            </Text>
            <ChevronDownIcon
              className={classNames(styles.chevron, {
                [styles.chevron__open]: !open,
                [styles.chevron__close]: open,
              })}
            />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          {toolCalls.map((toolCall) => {
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
