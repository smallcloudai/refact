import React from "react";
import { Text, Container, Box, Flex, Switch, Button } from "@radix-ui/themes";
import { type DiffChunk } from "../../events";
import { ScrollArea } from "../ScrollArea";
import SyntaxHighlighter from "react-syntax-highlighter";
import classNames from "classnames";

import styles from "./ChatContent.module.css";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import { type DiffChunkStatus } from "../../hooks";
import isEqual from "lodash.isequal";

export type DiffSumbitFunction = (
  operation: "add" | "remove",
  chunks: DiffChunkWithTypeAndApply[],
) => void;

function toDiff(str: string, type: "add" | "remove"): string {
  const sign = type === "add" ? "+" : "-";

  const replaceEscapedEOL = str
    .split("\n")
    .filter((_) => _)
    .join("\n" + sign);

  return sign + replaceEscapedEOL;
}

const _Highlight: React.FC<{
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

const Highlight = React.memo(_Highlight);

type DiffProps = {
  diff: DiffChunk;
  type?: "apply" | "unapply";
  value?: boolean;
  onChange?: (checked: boolean) => void;
};

export const Diff: React.FC<DiffProps> = ({ diff, type, value, onChange }) => {
  const removeString = diff.lines_remove && toDiff(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiff(diff.lines_add, "add");

  return (
    <Box>
      <Flex justify="between" align="center" p="1">
        <Text size="1">{diff.file_name}</Text>
        {type && (
          <Text as="label" size="1">
            {type}{" "}
            <Switch size="1" checked={value} onCheckedChange={onChange} />
          </Text>
        )}
      </Flex>
      <ScrollArea scrollbars="horizontal" asChild>
        <Flex className={styles.diff} py="2" direction="column">
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
        </Flex>
      </ScrollArea>
    </Box>
  );
};

export type DiffContentProps = {
  diffs: DiffChunk[];
  appliedChunks: DiffChunkStatus | null;
  onSubmit: DiffSumbitFunction;
};

export type DiffChunkWithTypeAndApply = DiffChunk & {
  type: "apply" | "unapply";
  apply: boolean;
};

function diffFormState(
  diffs: DiffChunk[],
  appliedChunks: number[],
): DiffChunkWithTypeAndApply[] {
  return diffs.map((diff, index) => {
    const type = appliedChunks.includes(index) ? "unapply" : "apply";
    return {
      type: type,
      apply: false,
      ...diff,
    };
  });
}

export const DiffContent: React.FC<DiffContentProps> = ({
  diffs,
  appliedChunks,
  onSubmit,
}) => {
  const status = React.useMemo(
    () => diffFormState(diffs, appliedChunks?.state ?? []),
    [appliedChunks?.state, diffs],
  );
  // TODO: handle loading
  // TODO: handle errors
  if (!appliedChunks?.state) {
    return (
      <Container>
        <Flex direction="column" display="inline-flex" maxWidth="100%">
          {diffs.map((diff, i) => (
            <Diff key={i} diff={diff} />
          ))}
        </Flex>
      </Container>
    );
  }
  return (
    <DiffForm
      onSubmit={onSubmit}
      diffs={status}
      canRemove={appliedChunks.state.length > 0}
    />
  );
};

const DiffForm: React.FC<{
  diffs: DiffChunkWithTypeAndApply[];
  onSubmit: (
    operation: "add" | "remove",
    chunks: DiffChunkWithTypeAndApply[],
  ) => void;
  canRemove: boolean;
}> = ({ diffs, onSubmit, canRemove }) => {
  const [state, setState] = React.useState<DiffChunkWithTypeAndApply[]>(diffs);
  const handleToggle = (index: number, checked: boolean) => {
    setState((prev) => {
      const next = prev.slice(0);
      if (!next[index]) return next;
      next[index] = { ...next[index], apply: checked };
      return next;
    });
  };

  const hasNotChanged = React.useMemo(() => {
    return isEqual(state, diffs);
  }, [state, diffs]);

  const addOp = React.useCallback(
    () => onSubmit("add", state),
    [onSubmit, state],
  );

  const removeOp = React.useCallback(
    () => onSubmit("remove", state),
    [onSubmit, state],
  );

  return (
    <Container>
      <Flex direction="column" display="inline-flex" maxWidth="100%">
        {state.map((diff, i) => (
          <Diff
            key={i}
            diff={diff}
            type={diff.type}
            value={diff.apply}
            onChange={(checked: boolean) => handleToggle(i, checked)}
          />
        ))}
      </Flex>
      <Button disabled={hasNotChanged} onClick={addOp}>
        Add
      </Button>
      <Button disabled={hasNotChanged || !canRemove} onClick={removeOp}>
        Remove
      </Button>
    </Container>
  );
};
