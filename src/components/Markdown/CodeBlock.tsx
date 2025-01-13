import React, { CSSProperties } from "react";
import SyntaxHighlighter, {
  type SyntaxHighlighterProps,
} from "react-syntax-highlighter";
import { Code, CodeProps, Text } from "@radix-ui/themes";
import classNames from "classnames";
import { PreTag, PreTagProps } from "./Pre";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import type { Element } from "hast";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import { trimIndent } from "../../utils";

export type MarkdownControls = {
  onCopyClick: (str: string) => void;
};

export type MarkdownCodeBlockProps = React.JSX.IntrinsicElements["code"] & {
  node?: Element | undefined;
  style?: Record<string, CSSProperties> | SyntaxHighlighterProps["style"];
  wrap?: boolean;
  color?: CodeProps["color"];
} & Pick<
    SyntaxHighlighterProps,
    "showLineNumbers" | "startingLineNumber" | "useInlineStyles"
  > &
  Partial<MarkdownControls>;

const _MarkdownCodeBlock: React.FC<MarkdownCodeBlockProps> = ({
  children,
  className,
  style = hljsStyle,
  onCopyClick,
  wrap = false,
  color = undefined,
}) => {
  const codeRef = React.useRef<HTMLElement | null>(null);
  const match = /language-(\w+)/.exec(className ?? "");
  const textWithOutTrailingNewLine = String(children).replace(/\n$/, "");
  const textWithOutIndent = trimIndent(textWithOutTrailingNewLine);

  const preTagProps: PreTagProps = onCopyClick
    ? {
        onCopyClick: () => {
          if (codeRef.current?.textContent) {
            onCopyClick(codeRef.current.textContent);
          }
        },
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
              className={classNames(
                styles.code,
                styles.code_block,
                wrap && styles.code_wrap,
              )}
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
    <Code
      className={classNames(styles.code, styles.code_inline, className)}
      color={color}
    >
      {children}
    </Code>
  );
};

export const MarkdownCodeBlock = React.memo(_MarkdownCodeBlock);
