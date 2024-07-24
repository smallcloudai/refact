import React from "react";
import {
  Text,
  Container,
  Box,
  Flex,
  // Switch,
  Button,
} from "@radix-ui/themes";
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
import { Reveal } from "../Reveal";

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
  canApply?: boolean;
  status?: number;
  value?: boolean;
  onChange?: (checked: boolean) => void;
};

const Diff: React.FC<DiffProps> = ({
  diff,
  status,
  canApply,
  // value,
  // onChange,
}) => {
  const removeString = diff.lines_remove && toDiff(diff.lines_remove, "remove");
  const addString = diff.lines_add && toDiff(diff.lines_add, "add");
  const title = filename(diff.file_name);
  const type =
    status === 2
      ? "error applying"
      : status === 1
        ? "applied"
        : canApply
          ? "apply"
          : "unapply";

  const lineCount =
    removeString.split("\n").length + addString.split("\n").length;
  return (
    <Box>
      <Flex justify="between" align="center" p="1">
        <Text size="1">{title}</Text>
        <Text size="1">{type}</Text>
        {/* {canApply && (
          <Text as="label" size="1">
            {type}{" "}
            {status !== 2 && (
              <Switch size="1" checked={value} onCheckedChange={onChange} />
            )}
          </Text> */}
      </Flex>
      <Reveal defaultOpen={lineCount < 9}>
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
      </Reveal>
    </Box>
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

const DiffsWithoutForm: React.FC<{ diffs: DiffChunk[] }> = ({ diffs }) => {
  return (
    <Flex direction="column" display="inline-flex" maxWidth="100%">
      {diffs.map((diff, i) => (
        <Diff key={i} diff={diff} />
      ))}
    </Flex>
  );
};

const DiffTitle: React.FC<{ diffs: DiffChunk[] }> = ({
  diffs,
}): React.ReactNode[] => {
  function process(
    diffs: DiffChunk[],
    memo: React.ReactNode[] = [],
  ): React.ReactNode[] {
    if (diffs.length === 0) return memo;
    const [head, ...tail] = diffs;
    const name = filename(head.file_name);
    const addLength = head.lines_add ? head.lines_add.split("\n").length : 0;
    const removeLength = head.lines_remove
      ? head.lines_remove.split("\n").length
      : 0;
    const adds = "+".repeat(addLength);
    const removes = "-".repeat(removeLength);
    const element = (
      <Text key={head.file_name + "-" + memo.length}>
        {name} <Text color="green">{adds}</Text>
        <Text color="red">{removes}</Text>
      </Text>
    );

    const nextMemo = memo.length > 0 ? [...memo, ", ", element] : [element];
    return process(tail, nextMemo);
  }

  return process(diffs);
};

export const DiffContent: React.FC<DiffContentProps> = ({
  diffs,
  appliedChunks,
  onSubmit,
}) => {
  const [open, setOpen] = React.useState(false);
  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="center">
            <Text weight="light" size="1">
              <DiffTitle diffs={diffs} />
            </Text>
            <Chevron open={open} />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          {!appliedChunks?.state ? (
            <DiffsWithoutForm diffs={diffs} />
          ) : (
            <DiffForm
              onSubmit={onSubmit}
              diffs={diffs}
              appliedChunks={appliedChunks}
            />
          )}
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

const DiffForm: React.FC<{
  diffs: DiffChunk[];
  appliedChunks: DiffChunkStatus;
  onSubmit: (chunks: boolean[]) => void;
}> = ({ diffs, onSubmit, appliedChunks }) => {
  const handleToggle = React.useCallback(
    (index: number, checked: boolean) => {
      const chunks = diffs.map((_, i) => {
        if (i === index) return checked;
        return appliedChunks.applied_chunks[i] || false;
      });
      onSubmit(chunks);
    },
    [appliedChunks.applied_chunks, diffs, onSubmit],
  );

  const disableApplyAll = React.useMemo(() => {
    if (appliedChunks.fetching) return true;
    return !appliedChunks.can_apply.every((_) => _);
  }, [appliedChunks.can_apply, appliedChunks.fetching]);

  const action = React.useMemo(() => {
    if (appliedChunks.applied_chunks.every((diff) => diff))
      return "Unapply All";
    return "Apply All";
  }, [appliedChunks.applied_chunks]);

  const applyAll = React.useCallback(() => {
    // const chunks = appliedChunks.applied_chunks.map((_) => true);
    // const toApply = appliedChunks.applied_chunks.map((_) => !_);
    const toApply = appliedChunks.can_apply;
    onSubmit(toApply);
  }, [appliedChunks.can_apply, onSubmit]);

  return (
    <Flex direction="column" display="inline-flex" maxWidth="100%">
      {diffs.map((diff, i) => {
        const canApply = appliedChunks.can_apply[i];
        const status = appliedChunks.state[i];
        const applied = status === 1 || appliedChunks.applied_chunks[i];
        return (
          <Diff
            key={i}
            diff={diff}
            status={status}
            canApply={canApply}
            value={applied}
            onChange={(checked: boolean) => handleToggle(i, checked)}
          />
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
