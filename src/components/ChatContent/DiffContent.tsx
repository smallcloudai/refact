import React from "react";
import { Text, Container, Box, Flex } from "@radix-ui/themes";
import { DiffAction } from "../../events";
import { Markdown } from "../Markdown";
import { ScrollArea } from "../ScrollArea";

import styles from "./ChatContent.module.css";

function toDiffMarkdown(str: string, type: "add" | "remove") {
  const replacement = type === "add" ? "\n+" : "\n-";
  const replaceEscapedEOL = str
    .split("\\n")
    .filter((_) => _)
    .join(replacement);

  return "```diff" + replacement + replaceEscapedEOL + "\n```";
}

// TODO: Add custom markdown compoents

export const Diff: React.FC<{ diff: DiffAction }> = ({ diff }) => {
  const removeString =
    diff.lines_remove && toDiffMarkdown(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiffMarkdown(diff.lines_add, "add");

  return (
    <Box>
      <Text size="1">{diff.file_name}</Text>
      <ScrollArea scrollbars="horizontal">
        <Box className={styles.diff}>
          <Markdown
            showLineNumbers={!!diff.line1}
            startingLineNumber={diff.line1}
          >
            {removeString}
          </Markdown>
          <Markdown
            showLineNumbers={!!diff.line1}
            startingLineNumber={diff.line1}
          >
            {addString}
          </Markdown>
        </Box>
      </ScrollArea>
    </Box>
  );
};

export const DiffContent: React.FC<{ diffs: DiffAction[] }> = ({ diffs }) => {
  return (
    <Container py="4">
      <Flex direction="column" display="inline-flex" maxWidth="100%">
        {diffs.map((diff, i) => (
          <Diff key={i} diff={diff} />
        ))}
      </Flex>
    </Container>
  );
};
