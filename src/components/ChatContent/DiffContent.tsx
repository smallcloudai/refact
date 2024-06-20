import React from "react";
import { Text, Container, Box, Flex } from "@radix-ui/themes";
import { DiffAction } from "../../events";
import { Markdown } from "../Markdown";
import { ScrollArea } from "../ScrollArea";

function replaceEscapedEOLWith(str: string, replacement: string) {
  return str
    .split("\\n")
    .filter((_) => _)
    .join(replacement);
}

export const Diff: React.FC<{ diff: DiffAction }> = ({ diff }) => {
  let diffString = "```diff\n";

  if (diff.lines_remove) {
    diffString += "-" + replaceEscapedEOLWith(diff.lines_remove, "\n-") + "\n";
  }
  if (diff.lines_add) {
    diffString += "+" + replaceEscapedEOLWith(diff.lines_add, "\n+") + "\n";
  }

  diffString += "\n```";

  return (
    <Box>
      <Text size="1">{diff.file_name}</Text>
      <ScrollArea scrollbars="horizontal">
        <Markdown>{diffString}</Markdown>
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
