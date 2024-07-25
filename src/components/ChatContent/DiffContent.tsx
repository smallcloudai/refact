import React from "react";
import { Text, Container, Box, Flex, Switch, Button } from "@radix-ui/themes";
import { type DiffChunk } from "../../events";
import { ScrollArea } from "../ScrollArea";
import SyntaxHighlighter from "react-syntax-highlighter";
import classNames from "classnames";

import styles from "./ChatContent.module.css";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import { type DiffChunkStatus } from "../../hooks";
import { filename } from "../../utils";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Chevron } from "../Collapsible";
import groupBy from "lodash.groupby";

type DiffType = "apply" | "unapply" | "error" | "can not apply";

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
};

const Diff: React.FC<DiffProps> = ({ diff }) => {
  const removeString = diff.lines_remove && toDiff(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiff(diff.lines_add, "add");
  return (
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
  );
};

export type DiffContentProps = {
  diffs: DiffChunk[];
  appliedChunks: DiffChunkStatus | null;
  onSubmit: (toApply: boolean[]) => void;
};

export type DiffChunkWithTypeAndApply = DiffChunk & {
  type: DiffType;
  apply: boolean;
};

const DiffsWithoutForm: React.FC<{ diffs: Record<string, DiffChunk[]> }> = ({
  diffs,
}) => {
  return (
    <Flex direction="column" maxWidth="100%" gap="2">
      {Object.entries(diffs).map(([fullFilePath, diffsForfile]) => {
        return (
          <Box key={fullFilePath}>
            <Text size="1" wrap="wrap">
              {fullFilePath}
            </Text>
            <ScrollArea scrollbars="horizontal" asChild>
              <Box style={{ background: "rgb(51, 51, 51)" }}>
                {diffsForfile.map((diff, index) => {
                  return (
                    <Diff diff={diff} key={diff.file_name + "-" + index} />
                  );
                })}
              </Box>
            </ScrollArea>
          </Box>
        );
      })}
    </Flex>
  );
};

const DiffTitle: React.FC<{ diffs: Record<string, DiffChunk[]> }> = ({
  diffs,
}): React.ReactNode[] => {
  const entries = Object.entries(diffs);

  function process(
    items: [string, DiffChunk[]][],
    memo: React.ReactNode[] = [],
  ): React.ReactNode[] {
    if (items.length === 0) return memo;
    const [head, ...tail] = items;
    const [fullPath, diffForFile] = head;
    const name = filename(fullPath);
    const addLength = diffForFile.reduce((acc, diff) => {
      return acc + (diff.lines_add ? diff.lines_add.split("\n").length : 0);
    }, 0);
    const removeLength = diffForFile.reduce((acc, diff) => {
      return (
        acc + (diff.lines_remove ? diff.lines_remove.split("\n").length : 0)
      );
    }, 0);
    const adds = "+".repeat(addLength);
    const removes = "-".repeat(removeLength);
    const element = (
      <Text
        style={{ display: "inline-block" }}
        key={fullPath + "-" + diffForFile.length}
      >
        {name} <Text color="green">{adds}</Text>
        <Text color="red">{removes}</Text>
      </Text>
    );
    const nextMemo = memo.length > 0 ? [...memo, ", ", element] : [element];

    return process(tail, nextMemo);
  }

  return process(entries);
};

export const DiffContent: React.FC<DiffContentProps> = ({
  diffs,
  appliedChunks,
  onSubmit,
}) => {
  const [open, setOpen] = React.useState(false);

  const groupedDiffs: Record<string, DiffWithStatus[]> = React.useMemo(() => {
    const diffWithStatus = diffs.map((diff, index) => {
      return {
        ...diff,
        state: appliedChunks?.state[index] ?? 0,
        can_apply: appliedChunks?.can_apply[index] ?? false,
        applied: appliedChunks?.applied_chunks[index] ?? false,
        index,
      };
    });

    return groupBy(diffWithStatus, (diff) => diff.file_name);
  }, [
    appliedChunks?.applied_chunks,
    appliedChunks?.can_apply,
    appliedChunks?.state,
    diffs,
  ]);

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="center">
            <Text weight="light" size="1">
              <DiffTitle diffs={groupedDiffs} />
            </Text>
            <Chevron open={open} />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          {!appliedChunks?.state ? (
            <DiffsWithoutForm diffs={groupedDiffs} />
          ) : (
            <DiffForm
              onSubmit={onSubmit}
              loading={appliedChunks.fetching}
              diffs={groupedDiffs}
            />
          )}
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

type DiffWithStatus = DiffChunk & {
  state: 0 | 1 | 2;
  can_apply: boolean;
  applied: boolean;
  index: number;
};

const DiffForm: React.FC<{
  diffs: Record<string, DiffWithStatus[]>;
  loading: boolean;
  onSubmit: (toApply: boolean[]) => void;
}> = ({ diffs, loading, onSubmit }) => {
  const values = React.useMemo(() => {
    return Object.values(diffs).reduce((acc, curr) => acc.concat(curr), []);
  }, [diffs]);

  const disableApplyAll = React.useMemo(() => {
    if (loading) return true;
    const allFalse = values.every((diff) => !diff.can_apply);
    const allTrue = values.every((diff) => diff.can_apply);
    return !(allTrue || allFalse);
  }, [loading, values]);

  const action = React.useMemo(() => {
    const canApply = values.map((diff) => diff.applied);
    const allApplied = canApply.every((diff) => diff);
    if (allApplied) return "Unapply All";
    return "Apply All";
  }, [values]);

  const applyAll = React.useCallback(() => {
    const ops = Object.values(diffs).reduce<boolean[]>((acc, diffs) => {
      const canApply = diffs.map((diff) => !diff.applied);
      return acc.concat(canApply);
    }, []);
    onSubmit(ops);
  }, [diffs, onSubmit]);

  const handleToggle = React.useCallback(
    (value: boolean, indeices: number[]) => {
      const toApply = values.map((diff, index) => {
        if (indeices.includes(index)) return value;
        return diff.applied;
      });
      onSubmit(toApply);
    },
    [onSubmit, values],
  );

  return (
    <Flex direction="column" maxWidth="100%" py="2" gap="2">
      {Object.entries(diffs).map(([fullFileName, diffsForFile], index) => {
        const key = fullFileName + "-" + index;
        const errored = diffsForFile.some((diff) => diff.state === 2);
        const applied = diffsForFile.every((diff) => diff.applied);
        const indeices = diffsForFile.map((diff) => diff.index);
        return (
          <Box key={key}>
            <Flex justify="between" align="center" p="1" gap="8">
              <Text size="1" wrap="wrap">
                {fullFileName}
              </Text>
              <Text size="1" as="label">
                <Flex align="center" gap="2">
                  {errored ? "error" : applied ? "unapply" : "apply"}{" "}
                  <Switch
                    disabled={loading || errored}
                    size="1"
                    checked={applied}
                    onCheckedChange={(value) => handleToggle(value, indeices)}
                  />
                </Flex>
              </Text>
            </Flex>
            {diffsForFile.map((diff, i) => (
              <Diff key={`${fullFileName}-${index}-${i}`} diff={diff} />
            ))}
          </Box>
        );
      })}

      <Flex gap="2" py="2">
        <Button disabled={disableApplyAll} onClick={applyAll}>
          {action}
        </Button>
      </Flex>
    </Flex>
  );
};
