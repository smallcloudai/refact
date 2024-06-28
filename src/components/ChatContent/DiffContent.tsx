import React from "react";
import { Text, Container, Box, Flex } from "@radix-ui/themes";
import { DiffAction } from "../../events";
import { ScrollArea } from "../ScrollArea";
import SyntaxHighlighter from "react-syntax-highlighter";
import classNames from "classnames";

import styles from "./ChatContent.module.css";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";

function toDiff(str: string, type: "add" | "remove"): string {
  const sign = type === "add" ? "+" : "-";

  const replaceEscapedEOL = str
    .split("\n")
    .filter((_) => _)
    .join("\n" + sign);

  return sign + replaceEscapedEOL;
}

const Highlight: React.FC<{
  children: string;
  showLineNumbers?: boolean;
  startingLineNumber?: number;
  className: string;
}> = ({ children, className, ...rest }) => {
  return (
    <SyntaxHighlighter
      style={hljsStyle}
      PreTag={(props) => (
        <pre {...props} className={classNames(styles.diff_pre, className)} />
      )}
      language="diff"
      {...rest}
    >
      {children}
    </SyntaxHighlighter>
  );
};

export const Diff: React.FC<{ diff: DiffAction }> = ({ diff }) => {
  const removeString = diff.lines_remove && toDiff(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiff(diff.lines_add, "add");

  return (
    <Box>
      <Text size="1">{diff.file_name}</Text>
      <ScrollArea scrollbars="horizontal">
        <Box className={styles.diff} py="2">
          {removeString && (
            <Highlight
              className={styles.diff_first}
              showLineNumbers={!!diff.line1}
              startingLineNumber={diff.line1}
            >
              {removeString}
            </Highlight>
          )}
          {addString && (
            <Highlight
              className={styles.diff_second}
              showLineNumbers={!!diff.line1}
              startingLineNumber={diff.line1}
            >
              {addString}
            </Highlight>
          )}
        </Box>
      </ScrollArea>
    </Box>
  );
};

export const DiffContent: React.FC<{ diffs: DiffAction[] }> = ({ diffs }) => {
  return (
    <Container>
      <Flex direction="column" display="inline-flex" maxWidth="100%">
        {diffs.map((diff, i) => (
          <Diff key={i} diff={diff} />
        ))}
      </Flex>
    </Container>
  );
};
