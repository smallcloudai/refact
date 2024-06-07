import React, { useMemo } from "react";

import { Markdown } from "./Markdown";

import * as Collapsible from "@radix-ui/react-collapsible";
import { Box, Flex } from "@radix-ui/themes";
import { Cross2Icon, RowSpacingIcon } from "@radix-ui/react-icons";
import styles from "./CommandLine.module.css";
import { ScrollArea } from "../ScrollArea";

export type CommandLineProps = {
  command: string;
  args: string;
  error?: boolean;
  result: string;
};

export const CommandLine: React.FC<CommandLineProps> = ({
  command,
  args,
  error: _error, // TODO: style errors
  result,
}) => {
  const argsString = useMemo(() => {
    try {
      const json = JSON.parse(args) as unknown as Parameters<
        typeof Object.entries
      >;
      if (Array.isArray(json)) {
        return json.join(", ");
      }
      return Object.entries(json)
        .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
        .join(", ");
    } catch {
      return args;
    }
  }, [args]);

  const str = "```python\n" + command + "(" + argsString + ")\n```";

  const escapedBackticks = result.replace(/`+/g, (match) => {
    if (match === "```") return match;
    return "\\" + "`";
  });

  const [open, setOpen] = React.useState(false);
  return (
    <Box m="2">
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex className={styles.button}>
            <ScrollArea scrollbars="horizontal">
              <Markdown className={styles.command}>{str}</Markdown>
            </ScrollArea>
            <div
              style={{
                right: "var(--space-3)",
                top: "var(--space-1)",
                position: "absolute",
              }}
            >
              {open ? <Cross2Icon /> : <RowSpacingIcon />}
            </div>
          </Flex>
        </Collapsible.Trigger>

        <Collapsible.Content className={styles.content}>
          <ScrollArea scrollbars="horizontal">
            <Markdown>{escapedBackticks}</Markdown>
          </ScrollArea>
        </Collapsible.Content>
      </Collapsible.Root>
    </Box>
  );
};
