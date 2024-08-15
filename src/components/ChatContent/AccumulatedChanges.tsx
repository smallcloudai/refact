import React from "react";
import { Container, Text, Flex, Box, Button } from "@radix-ui/themes";
import { ChatMessages, DiffChunk, isDiffMessage } from "../../events";
import { type DiffWithStatus, DiffTitle, Diff } from "./DiffContent";
import { DiffChunkStatus } from "../../hooks";
import groupBy from "lodash.groupby";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Chevron } from "../Collapsible";
import { TruncateLeft } from "../Text";
import { ScrollArea } from "../ScrollArea";

export const AccumulatedChanges: React.FC<{
  messages: ChatMessages;
  getDiffByIndex: (index: string) => DiffChunkStatus | null;
  onSumbit: (args: {
    diff_id: string;
    chunks: DiffChunk[];
    toApply: boolean[];
  }) => void;
}> = ({ messages, onSumbit, getDiffByIndex }) => {
  // TODO: bug where it keeps loading state.
  const [open, setOpen] = React.useState(false);

  const diffs = React.useMemo(() => {
    return messages.reduce<(DiffWithStatus & { tool_call_id: string })[]>(
      (acc, cur) => {
        if (!isDiffMessage(cur)) return acc;
        const stats = getDiffByIndex(cur[2]);
        const diffs = cur[1].map((diff, index) => {
          return {
            ...diff,
            tool_call_id: cur[2],
            applied: stats?.applied_chunks[index] ?? false,
            can_apply: stats?.can_apply[index] ?? false,
            state: stats?.state[index] ?? 0,
            index,
          };
        });
        return acc.concat(diffs);
      },
      [],
    );
  }, [getDiffByIndex, messages]);

  const loading = React.useMemo(() => {
    return diffs.some((diff) => {
      const status = getDiffByIndex(diff.tool_call_id);
      return status?.fetching === true;
    }, []);
  }, [getDiffByIndex, diffs]);

  const groupedDiffs = groupBy(diffs, (diff) => diff.file_name);

  if (Object.entries(groupedDiffs).length === 0) return null;

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Box>
            <Flex direction="row" align="center" gap="2">
              <Text size="1">Accumulated changes</Text>
              <Chevron open={open} />
            </Flex>
            <Text size="1" wrap="wrap">
              <DiffTitle diffs={groupedDiffs} />
            </Text>
          </Box>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <DiffForm
            diffs={groupedDiffs}
            loading={loading}
            onSubmit={onSumbit}
          />
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

type DiffWithStatusWithCallId = DiffWithStatus & { tool_call_id: string };

const DiffForm: React.FC<{
  diffs: Record<string, DiffWithStatusWithCallId[]>;
  loading: boolean;
  onSubmit: (args: {
    diff_id: string;
    chunks: DiffChunk[];
    toApply: boolean[];
  }) => void;
}> = ({ diffs, loading, onSubmit }) => {
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
    // const canApply = values.map((diff) => diff.applied);
    const allApplied = values.every((diff) => diff.applied);
    if (allApplied) return "Unapply All";
    return "Apply All";
  }, [values]);

  const applyAll = React.useCallback(() => {
    const groupedByToolCall = groupBy(values, (diff) => diff.tool_call_id);
    Object.entries(groupedByToolCall).forEach(([toolCallId, diffs]) => {
      const toApply = diffs.map((diff) => !diff.applied);
      onSubmit({ diff_id: toolCallId, chunks: diffs, toApply });
    });
  }, [onSubmit, values]);

  const handleToggle = React.useCallback(
    (value: boolean, diffsForFile: DiffWithStatusWithCallId[]) => {
      const d = groupBy(diffsForFile, (diff) => diff.tool_call_id);
      Object.entries(d).forEach(([id, diffsForToolCall]) => {
        const toApply = diffsForToolCall.map((diff) => {
          if (diff.tool_call_id === id) return value;
          return diff.applied;
        });

        const chunks: DiffChunk[] = diffsForToolCall.map((diff) => {
          const {
            state: _state,
            tool_call_id: _tool_call_id,
            applied: _applied,
            can_apply: _can_apply,
            ...chunk
          } = diff;
          return chunk;
        });
        onSubmit({ diff_id: id, chunks, toApply });
      });
    },
    [onSubmit],
  );

  return (
    <Flex direction="column" maxWidth="100%" py="2" gap="2">
      {Object.entries(diffs).map(([fullFileName, diffsForFile], index) => {
        const key = fullFileName + "-" + index;
        const errored = diffsForFile.some((diff) => diff.state === 2);
        const applied = diffsForFile.every((diff) => diff.applied);
        return (
          <Box key={key} my="2">
            <Flex justify="between" align="center" p="1">
              <TruncateLeft size="1">{fullFileName}</TruncateLeft>
              <Text size="1" as="label">
                <Flex align="center" gap="2" pl="2">
                  {errored && "error"}
                  <Button
                    size="1"
                    disabled={loading}
                    onClick={() => {
                      handleToggle(!applied, diffsForFile);
                    }}
                  >
                    {applied ? "Unapply" : "Apply"}
                  </Button>
                </Flex>
              </Text>
            </Flex>
            <ScrollArea scrollbars="horizontal" asChild>
              <Box style={{ background: "rgb(51, 51, 51)" }}>
                {diffsForFile.map((diff, i) => (
                  <Diff key={`${fullFileName}-${index}-${i}`} diff={diff} />
                ))}
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
