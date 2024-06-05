import React from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Container, Flex, Text } from "@radix-ui/themes";
import { ToolCall, ToolResult } from "../../events";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import styles from "./ChatContent.module.css";
import { Markdown } from "../CommandLine/Markdown";

export const Tool: React.FC<{
  toolCall: ToolCall;
  result: ToolResult;
}> = ({ toolCall, result }) => {
  const [open, setOpen] = React.useState(false);

  const name = toolCall.function.name ?? "";
  const type = toolCall.type ?? "function";

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
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" pb="2" align="center">
            <Text weight="light" size="1">
              Called {type} {name}
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
          <Markdown>{functionCalled + "\n" + result.content}</Markdown>
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};
