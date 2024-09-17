import React from "react";
import { Text, Container, Box, Flex, Button, Link } from "@radix-ui/themes";
import { isDiffErrorResponseData, type DiffChunk } from "../../services/refact";
import { ScrollArea } from "../ScrollArea";
import styles from "./ChatContent.module.css";
import { filename } from "../../utils";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Chevron } from "../Collapsible";
import groupBy from "lodash.groupby";
import { TruncateLeft } from "../Text";
import {
  useDiffApplyMutation,
  useDiffStateQuery,
  useConfig,
  useDiffPreview,
  useAppDispatch,
  useAppSelector,
} from "../../hooks";

// import { setError, clearError } from "../../features/Errors/errorsSlice";
import {
  clearWarning,
  getWarningMessage,
  setWarning,
} from "../../features/Errors/warningSlice";
import { ErrorCallout } from "../Callout";

type DiffType = "apply" | "unapply" | "error" | "can not apply";

function toDiff(str: string): string {
  const replaceEscapedEOL = str
    .split("\n")
    .filter((_) => _)
    .join("\n");

  return replaceEscapedEOL;
}

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
  const removeString = diff.lines_remove && toDiff(diff.lines_remove);
  const addString = diff.lines_add && toDiff(diff.lines_add);
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

export const DiffTitle: React.FC<{ diffs: Record<string, DiffChunk[]> }> = ({
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
        <Text color="red" wrap="wrap">
          {removes}
        </Text>
        <Text color="green" wrap="wrap">
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
  chunks: DiffChunk[];
  toolCallId: string;
}> = ({ chunks, toolCallId }) => {
  const [open, setOpen] = React.useState(false);

  const diffStateRequest = useDiffStateQuery({ chunks, toolCallId });

  const { onPreview, previewResult: _previewResult } = useDiffPreview(chunks);

  const { onSubmit, result: _result } = useDiffApplyMutation();
  const dispatch = useAppDispatch();

  const groupedDiffs: Record<string, DiffWithStatus[]> = React.useMemo(() => {
    const diffWithStatus = chunks.map((diff, index) => {
      return {
        ...diff,
        // state: result.data?.state[index] ?? 0,
        can_apply: diffStateRequest.data?.can_apply[index] ?? false,
        applied: diffStateRequest.data?.state[index] ?? false,
        index,
      };
    });

    return groupBy(diffWithStatus, (diff) => diff.file_name);
  }, [chunks, diffStateRequest]);

  const handleDiffApplySubmit = (toApply: boolean[]) => {
    onSubmit({ chunks, toApply, toolCallId })
      .unwrap()
      .then((payload) => {
        let data = null;
        if (!Array.isArray(payload)) {
          return;
        }
        data = payload[0];

        if (isDiffErrorResponseData(data)) {
          if (data.detail) {
            const [warning, filePath] = data.detail.split("\n")[0].split("'");
            const normalizedPath = filePath.startsWith("\\\\?\\")
              ? filePath.substring(4).replace(/\\/g, "/")
              : filePath;

            const reason = data.detail.split("\n")[1];
            dispatch(setWarning([[warning, normalizedPath].join(" "), reason]));
          }
        }
      })
      .catch((error) => dispatch(setWarning(error as string[])));
  };

  // if (diffStateRequest.isFetching) return null;
  // if (diffStateRequest.isError) return null;
  // if (!diffStateRequest.data) return null;

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
          <DiffForm
            onSubmit={handleDiffApplySubmit}
            onPreview={onPreview}
            loading={diffStateRequest.isLoading}
            diffs={groupedDiffs}
            openFile={() => {
              // TODO:
            }}
          />
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

export type DiffWithStatus = DiffChunk & {
  state?: 0 | 1 | 2;
  can_apply: boolean;
  applied: boolean;
  index: number;
};

export const DiffForm: React.FC<{
  diffs: Record<string, DiffWithStatus[]>;
  loading: boolean;
  onSubmit: (toApply: boolean[]) => void;
  onPreview: (toApply: boolean[]) => void | Promise<void>;
  openFile: (file: { file_name: string; line?: number }) => void;
}> = ({ diffs, loading, onSubmit, onPreview, openFile }) => {
  const dispatch = useAppDispatch();
  const warning = useAppSelector(getWarningMessage);
  const onClearWarning = React.useCallback(
    () => dispatch(clearWarning()),
    [dispatch],
  );

  const { host } = useConfig();
  const values = React.useMemo(() => {
    return Object.values(diffs).reduce((acc, curr) => acc.concat(curr), []);
  }, [diffs]);

  const disableApplyAll = React.useMemo(() => {
    if (loading) return true;
    const start = values[0]?.applied ?? false;
    const allTheSame = values.every((diff) => diff.applied === start);
    const allCanApply = values.every((diff) => diff.can_apply);
    return !(allTheSame && allCanApply);
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
    (value: boolean, indices: number[]) => {
      const toApply = values.map((diff, index) => {
        if (indices.includes(index)) return value;
        return diff.applied;
      });
      onSubmit(toApply);
    },
    [onSubmit, values],
  );

  const handlePreview = React.useCallback(
    (value: boolean, indices: number[]) => {
      const toApply = values.map((diff, index) => {
        if (indices.includes(index)) return value;
        return diff.applied;
      });
      void onPreview(toApply);
    },
    [values, onPreview],
  );

  return (
    <Flex direction="column" maxWidth="100%" py="2" gap="2">
      {Object.entries(diffs).map(([fullFileName, diffsForFile], index) => {
        const key = fullFileName + "-" + index;
        const errored = diffsForFile.some((diff) => diff.state === 2);
        const applied = diffsForFile.every((diff) => diff.applied);
        const indices = diffsForFile.map((diff) => diff.index);
        return (
          <Box key={key} my="2">
            <Flex justify="between" align="center" p="1">
              <TruncateLeft size="1">
                <Link
                  href="#"
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

              <Text size="1" as="label">
                <Flex align="center" gap="2" pl="2">
                  {errored && "error"}
                  {host === "vscode" && (
                    <Button
                      size="1"
                      disabled={loading}
                      onClick={() => handlePreview(!applied, indices)}
                    >
                      Preview
                    </Button>
                  )}
                  <Button
                    size="1"
                    disabled={loading}
                    onClick={() => handleToggle(!applied, indices)}
                  >
                    {applied ? "Unapply" : "Apply"}
                  </Button>
                </Flex>
              </Text>
            </Flex>
            <ScrollArea scrollbars="horizontal" asChild>
              <Box style={{ minWidth: "100%", position: "relative" }}>
                {warning && warning.length !== 0 && (
                  <ErrorCallout
                    onClick={onClearWarning}
                    timeout={null}
                    itemType="warning"
                    my="4"
                    message={warning}
                  >
                    <Text size="1" as="div" mt="1">
                      Click to retry
                    </Text>
                  </ErrorCallout>
                )}
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

      <Flex gap="2" py="2">
        <Button disabled={disableApplyAll || loading} onClick={applyAll}>
          {action}
        </Button>
      </Flex>
    </Flex>
  );
};
