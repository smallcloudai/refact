import React from "react";
import { Text, Container, Box, Flex, Link } from "@radix-ui/themes";
import { DiffMessage, type DiffChunk } from "../../services/refact";
import { ScrollArea } from "../ScrollArea";
import styles from "./ChatContent.module.css";
import { filename } from "../../utils";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Chevron } from "../Collapsible";
import groupBy from "lodash.groupby";
import { TruncateLeft } from "../Text";
import { useEventsBusForIDE } from "../../hooks";

type DiffType = "apply" | "unapply" | "error" | "can not apply";

const DiffLine: React.FC<{
  lineNumber?: number;
  sign: string;
  line: string;
}> = ({ lineNumber, sign, line }) => {
  const backgroundColorLeft = sign === "-" ? "#592e30" : "#3b5840";
  const backgroundColor = sign === "-" ? "#3e2628" : "#2c3e33";
  return (
    <Flex className={styles.diff_line} style={{ minWidth: "min-content" }}>
      <Text
        size="2"
        className={styles.diff_line_number}
        style={{ backgroundColor: backgroundColorLeft }}
      >
        {lineNumber ?? ""}
      </Text>
      <Text size="2" className={styles.diff_sign} style={{ backgroundColor }}>
        {sign}
      </Text>
      <Text
        size="2"
        className={styles.diff_line_content}
        style={{
          backgroundColor,
          whiteSpace: "pre",
          whiteSpaceTrim: "none",
          minWidth: "min-content",
        }}
      >
        {line}
      </Text>
    </Flex>
  );
};

const DiffHighlight: React.FC<{
  startLine?: number;
  sign: string;
  text: string;
}> = ({ startLine, sign, text }) => {
  const lines = text.split("\n");
  return (
    <Flex
      direction="column"
      style={{ minWidth: "min-content", alignSelf: "stretch", width: "100%" }}
    >
      {lines.map((line, index) => {
        return (
          <DiffLine
            key={index}
            line={line}
            sign={sign}
            lineNumber={startLine ? index + startLine : undefined}
          />
        );
      })}
    </Flex>
  );
};

type DiffProps = {
  diff: DiffChunk;
};

export const Diff: React.FC<DiffProps> = ({ diff }) => {
  const removeString = diff.lines_remove && diff.lines_remove;
  const addString = diff.lines_add && diff.lines_add;
  return (
    <Flex
      className={styles.diff}
      py="2"
      direction="column"
      style={{ minWidth: "min-content" }}
    >
      {removeString && (
        <DiffHighlight startLine={diff.line1} sign={"-"} text={removeString} />
      )}
      {addString && (
        <DiffHighlight startLine={diff.line1} sign={"+"} text={addString} />
      )}
    </Flex>
  );
};

export type DiffChunkWithTypeAndApply = DiffChunk & {
  type: DiffType;
  apply: boolean;
};

export const DiffTitle: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }): React.ReactNode[] => {
  const entries = Object.entries(diffs);

  function process(
    items: [string, DiffChunk[]][],
    memo: React.ReactNode[] = [],
  ): React.ReactNode[] {
    if (items.length === 0) return memo;
    const [head, ...tail] = items;
    const [fullPath, diffForFile] = head;
    const name = filename(fullPath);
    const addLength = diffForFile.reduce<number>((acc, diff) => {
      return acc + (diff.lines_add ? diff.lines_add.split("\n").length : 0);
    }, 0);
    const removeLength = diffForFile.reduce<number>((acc, diff) => {
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
        {name}{" "}
        <Text color="red" wrap="wrap" style={{ wordBreak: "break-all" }}>
          {removes}
        </Text>
        <Text color="green" wrap="wrap" style={{ wordBreak: "break-all" }}>
          {adds}
        </Text>
      </Text>
    );
    const nextMemo = memo.length > 0 ? [...memo, ", ", element] : [element];

    return process(tail, nextMemo);
  }

  return process(entries);
};

export const DiffContent: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }) => {
  const [open, setOpen] = React.useState(false);

  return (
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
        <DiffForm diffs={diffs} />
      </Collapsible.Content>
    </Collapsible.Root>
  );
};

export type DiffWithStatus = DiffChunk & {
  state?: 0 | 1 | 2;
  can_apply: boolean;
  applied: boolean;
  index: number;
};

export const DiffForm: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }) => {
  const { openFile } = useEventsBusForIDE();
  return (
    <Flex direction="column" maxWidth="100%" py="2" gap="2">
      {Object.entries(diffs).map(([fullFileName, diffsForFile], index) => {
        const key = fullFileName + "-" + index;

        return (
          <Box key={key} my="2">
            <Flex justify="between" align="center" p="1">
              <TruncateLeft size="1">
                <Link
                  // TODO: check how ides treat this being "", undefined, or "#"
                  href=""
                  onClick={(event) => {
                    event.preventDefault();
                    const startLine = Math.min(
                      ...diffsForFile.map((diff) => diff.line1),
                    );
                    openFile({
                      file_name: fullFileName,
                      line: startLine,
                    });
                  }}
                >
                  {fullFileName}
                </Link>
              </TruncateLeft>
            </Flex>
            <ScrollArea scrollbars="horizontal" asChild>
              <Box style={{ minWidth: "100%", position: "relative" }}>
                <Box
                  style={{
                    background: "rgb(51, 51, 51)",
                    minWidth: "min-content",
                  }}
                >
                  {diffsForFile.map((diff, i) => (
                    <Diff key={`${fullFileName}-${index}-${i}`} diff={diff} />
                  ))}
                </Box>
              </Box>
            </ScrollArea>
          </Box>
        );
      })}
    </Flex>
  );
};

type GroupedDiffsProps = {
  diffs: DiffMessage[];
};

export const GroupedDiffs: React.FC<GroupedDiffsProps> = ({ diffs }) => {
  const chunks = diffs.reduce<DiffMessage["content"]>(
    (acc, diff) => [...acc, ...diff.content],
    [],
  );

  const groupedByFileName = groupBy(chunks, (chunk) => chunk.file_name);

  return (
    <Container>
      <Flex direction="column" gap="4" py="4">
        <DiffContent diffs={groupedByFileName} />
      </Flex>
    </Container>
  );
};
