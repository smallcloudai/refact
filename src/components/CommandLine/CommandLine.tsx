import React from "react";

import { Markdown } from "./Markdown";

import * as Collapsible from "@radix-ui/react-collapsible";
import { Box, Button } from "@radix-ui/themes";
import { Cross2Icon, RowSpacingIcon } from "@radix-ui/react-icons";
import styles from "./CommandLine.module.css";

export type CommandLineProps = {
  command: string;
  args: Record<string, string>;
  error?: boolean;
  result: string;
};

export const CommandLine: React.FC<CommandLineProps> = ({
  command,
  args,
  error: _error, // TODO: style errors
  result,
}) => {
  const argsString = Object.values(args).join(" ");

  const str = "```bash\n" + command + " " + argsString + "\n```";

  const [open, setOpen] = React.useState(false);
  return (
    <Box m="3">
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Button size="2" variant="ghost" className={styles.button}>
            <Markdown className={styles.command}>{str}</Markdown>
            <div style={{ right: "var(--space-3)", position: "absolute" }}>
              {open ? <Cross2Icon /> : <RowSpacingIcon />}
            </div>
          </Button>
        </Collapsible.Trigger>

        <Collapsible.Content className={styles.content}>
          <Markdown>{result}</Markdown>
        </Collapsible.Content>
      </Collapsible.Root>
    </Box>
  );
};
