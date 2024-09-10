import React, { CSSProperties } from "react";
import SyntaxHighlighter, {
  type SyntaxHighlighterProps,
} from "react-syntax-highlighter";
import { Code, Text } from "@radix-ui/themes";
import classNames from "classnames";
import { PreTag, type PreTagProps } from "./Pre";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import type { Element } from "hast";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import { trimIndent } from "../../utils";
import { DiffChunk } from "../../services/refact/types";
import { useDiffPreview } from "../../hooks";

export type MarkdownControls = {
  onCopyClick: (str: string) => void;
  onNewFileClick: (str: string) => void;
  onPasteClick: (str: string) => void;
  canPaste: boolean;
};

function convertMarkdownToDiffChunk(markdown: string): DiffChunk {
  const lines = markdown.split("\n");
  let fileName = "";
  let fileAction = ""; // file_action must be one of `edit, add, rename, remove`
  let line1 = 0;
  let line2 = 0;
  let linesRemove = "";
  let linesAdd = "";

  lines.forEach((line) => {
    if (line.startsWith("--- ")) {
      fileName = line.substring(4).trim(); // Extract file name from the line
      fileAction = "remove"; // Action for the original file
    } else if (line.startsWith("+++ ")) {
      fileName = line.substring(4).trim(); // Extract file name from the line
      fileAction = fileAction === "remove" ? "edit" : "add"; // Action for the new file
    } else if (line.startsWith("@@ ")) {
      const parts = line.match(/@@ -(\d+),\d+ \+(\d+),\d+ @@/);
      if (parts) {
        line1 = parseInt(parts[1], 10); // Starting line number for the original file
        line2 = parseInt(parts[2], 10); // Starting line number for the new file
      }
    } else if (line.startsWith("-")) {
      linesRemove += line.substring(1).trim() + "\n"; // Lines removed
    } else if (line.startsWith("+")) {
      linesAdd += line.substring(1).trim() + "\n"; // Lines added
    }
  });

  return {
    file_name: fileName,
    file_action: fileAction,
    line1: line1,
    line2: line2,
    lines_remove: linesRemove.trim(),
    lines_add: linesAdd.trim(),
  };
}

function useDiff(language: string, markdown: string) {
  const isDiff = language === "language-diff";
  const chunk = convertMarkdownToDiffChunk(markdown);
  const { onPreview } = useDiffPreview([chunk]);
  return { onPreview, isDiff };
}

export type MarkdownCodeBlockProps = React.JSX.IntrinsicElements["code"] &
  Partial<MarkdownControls> & {
    node?: Element | undefined;
    style?: Record<string, CSSProperties> | SyntaxHighlighterProps["style"];
  } & Pick<
    SyntaxHighlighterProps,
    "showLineNumbers" | "startingLineNumber" | "useInlineStyles"
  >;

const _MarkdownCodeBlock: React.FC<MarkdownCodeBlockProps> = ({
  children,
  className,
  onCopyClick,
  onNewFileClick,
  onPasteClick,
  canPaste,
  style = hljsStyle,
}) => {
  const codeRef = React.useRef<HTMLElement | null>(null);
  const match = /language-(\w+)/.exec(className ?? "");
  const textWithOutTrailingNewLine = String(children); //.replace(/\n$/, "");
  const textWithOutIndent = trimIndent(textWithOutTrailingNewLine);
  const { isDiff, onPreview } = useDiff(
    className ?? "",
    textWithOutTrailingNewLine,
  );
  const preTagProps: PreTagProps =
    onCopyClick && onNewFileClick && onPasteClick
      ? {
          onCopyClick: () => {
            if (codeRef.current?.textContent) {
              onCopyClick(codeRef.current.textContent);
            }
          },
          onNewFileClick: () => {
            if (codeRef.current?.textContent) {
              onNewFileClick(codeRef.current.textContent);
            }
          },
          onPasteClick: () => {
            if (isDiff) {
              void onPreview([true]);
            } else if (codeRef.current?.textContent) {
              onPasteClick(codeRef.current.textContent);
            }
          },
          canPaste: !!canPaste,
        }
      : {};

  if (match ?? String(children).includes("\n")) {
    const language: string = match && match.length > 0 ? match[1] : "text";
    return (
      <Text size="2">
        <SyntaxHighlighter
          style={style}
          className={className}
          PreTag={(props) => <PreTag {...props} {...preTagProps} />}
          CodeTag={(props) => (
            <Code
              {...props}
              className={classNames(styles.code, styles.code_block)}
              ref={codeRef}
            />
          )}
          language={language}
          // useInlineStyles={false}
        >
          {textWithOutIndent}
        </SyntaxHighlighter>
      </Text>
    );
  }

  return (
    <Code className={classNames(styles.code, styles.code_inline, className)}>
      {children}
    </Code>
  );
};

export const MarkdownCodeBlock = React.memo(_MarkdownCodeBlock);
