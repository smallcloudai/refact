import React from "react";
import { Text, Container, Box, Flex } from "@radix-ui/themes";
import { DiffAction } from "../../events";
import { ScrollArea } from "../ScrollArea";
import SyntaxHighlighter, {
  type SyntaxHighlighterProps,
} from "react-syntax-highlighter";
import classNames from "classnames";
import ReactMarkdown from "react-markdown";
import { type Element } from "hast";
import styles from "./ChatContent.module.css";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";

function toDiffMarkdown(str: string, type: "add" | "remove") {
  const replacement = type === "add" ? "\n+" : "\n-";
  const replaceEscapedEOL = str
    .split("\\n")
    .filter((_) => _)
    .join(replacement);

  return "```diff" + replacement + replaceEscapedEOL + "\n```";
}

// TODO: add this to the Markdown components

type CodeBlockProps = React.JSX.IntrinsicElements["code"] & {
  node?: Element | undefined;
  style?: Record<string, React.CSSProperties>;
} & Pick<
    SyntaxHighlighterProps,
    "showLineNumbers" | "startingLineNumber" | "style"
  >;

const CodeBlock: React.FC<CodeBlockProps> = ({
  children,
  className,
  color: _color,
  ref: _ref,
  node: _node,
  ...rest
}) => {
  const match = /language-(\w+)/.exec(className ?? "");
  const textWithOutTrailingNewLine = String(children).replace(/\n$/, "");

  const language: string = match && match.length > 0 ? match[1] : "text";
  return (
    <SyntaxHighlighter
      className={className}
      PreTag={(props) => (
        <pre {...props} className={classNames(styles.diff_pre)} />
      )}
      language={language}
      {...rest}
    >
      {textWithOutTrailingNewLine.trim()}
    </SyntaxHighlighter>
  );
};

type MarkdownProps = {
  children?: string;
  className?: string;
} & Pick<CodeBlockProps, "showLineNumbers" | "startingLineNumber">;

const Markdown: React.FC<MarkdownProps> = ({
  children,
  className,
  ...rest
}) => {
  return (
    <ReactMarkdown
      className={className}
      components={{
        code: (props) => <CodeBlock {...props} {...rest} style={hljsStyle} />,
      }}
    >
      {children}
    </ReactMarkdown>
  );
};

export const Diff: React.FC<{ diff: DiffAction }> = ({ diff }) => {
  const removeString =
    diff.lines_remove && toDiffMarkdown(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiffMarkdown(diff.lines_add, "add");

  return (
    <Box>
      <Text size="1">{diff.file_name}</Text>
      <ScrollArea scrollbars="horizontal">
        <Box className={styles.diff} py="4">
          <Markdown
            className={styles.diff_first}
            showLineNumbers={!!diff.line1}
            startingLineNumber={diff.line1}
          >
            {removeString}
          </Markdown>
          <Markdown
            className={styles.diff_second}
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
