import React from "react";
import { Container, Text, Flex, Box, Button } from "@radix-ui/themes";
import {
  ChatMessages,
  DiffAppliedStateArgs,
  DiffChunk,
  isDiffMessage,
} from "../../events";
import { DiffTitle, Diff } from "./DiffContent";
import groupBy from "lodash.groupby";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Chevron } from "../Collapsible";
import { TruncateLeft } from "../Text";
import { ScrollArea } from "../ScrollArea";
import { useDiffApplyMutation, useGetManyDiffState } from "../../app/hooks";

export const AccumulatedChanges: React.FC<{ messages: ChatMessages }> = ({
  messages,
}) => {
  const [open, setOpen] = React.useState(false);
  const [onSumbit, result] = useDiffApplyMutation();

  const handleSubmit = React.useCallback(
    (chunks: DiffChunk[], toApply: boolean[], toolCallId: string) => {
      void onSumbit({ chunks, toApply, toolCallId });
    },
    [onSumbit],
  );

  const args = React.useMemo(() => {
    return messages.reduce<DiffAppliedStateArgs[]>((acc, diff) => {
      if (!isDiffMessage(diff)) return acc;
      const [, chunks, toolCallId] = diff;
      const arg: DiffAppliedStateArgs = { chunks, toolCallId };
      return [...acc, arg];
    }, []);
  }, [messages]);

  const { allDiffRequest } = useGetManyDiffState(args);

  const loading = React.useMemo(() => {
    if (result.isLoading) return true;
    return allDiffRequest.some((diff) => diff.isLoading);
  }, [allDiffRequest, result]);

  const diffs = React.useMemo(() => {
    return allDiffRequest.reduce<ChunkWithMetaInfo[]>((acc, curr) => {
      if (!curr.originalArgs) return acc;
      const { chunks, toolCallId } = curr.originalArgs;
      const diffs = chunks.map((chunk, index) => {
        return {
          chunk,
          toolCallId: toolCallId,
          applied: curr.data?.state[index] ?? false,
          can_apply: curr.data?.can_apply[index] ?? false,
          index,
          // state: 0,
        };
      });
      return acc.concat(diffs);
    }, []);
  }, [allDiffRequest]);

  const groupedByFile = React.useMemo(
    () => groupBy(diffs, (diff) => diff.chunk.file_name),
    [diffs],
  );

  const diffForTile = React.useMemo(() => {
    return Object.entries(groupedByFile).reduce<Record<string, DiffChunk[]>>(
      (acc, [key, value]) => {
        const chunks = value.reduce<DiffChunk[]>(
          (acc, curr) => acc.concat(curr.chunk),
          [],
        );

        return {
          ...acc,
          [key]: chunks,
        };
      },
      {},
    );
  }, [groupedByFile]);

  if (diffs.length === 0) return null;

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
              <DiffTitle diffs={diffForTile} />
            </Text>
          </Box>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <DiffForm
            diffs={groupedByFile}
            loading={loading}
            onSubmit={handleSubmit}
          />
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

type ChunkWithMetaInfo = {
  chunk: DiffChunk;
  index: number;
  toolCallId: string;
  applied: boolean;
  can_apply: boolean;
};

const DiffForm: React.FC<{
  diffs: Record<string, ChunkWithMetaInfo[]>;
  loading: boolean;
  onSubmit: (
    chunks: DiffChunk[],
    toApply: boolean[],
    toolCallId: string,
  ) => void;
}> = ({ diffs, loading, onSubmit }) => {
  const values = React.useMemo(() => {
    return Object.values(diffs).reduce((acc, curr) => acc.concat(curr), []);
  }, [diffs]);

  const canApplyAll = React.useMemo(() => {
    if (values.length === 0) return false;

    const start = values[0]?.applied ?? false;
    const allTheSame = values.every((diff) => diff.applied === start);
    const allCanApply = values.every((diff) => diff.can_apply);
    return allTheSame && allCanApply;
  }, [values]);

  const action = React.useMemo(() => {
    // const canApply = values.map((diff) => diff.applied);
    const allApplied = values.every((diff) => diff.applied);
    if (allApplied) return "Unapply All";
    return "Apply All";
  }, [values]);

  const handleToggle = React.useCallback(
    (value: boolean, diffsForFile: ChunkWithMetaInfo[]) => {
      const d = groupBy(diffsForFile, (diff) => diff.toolCallId);
      Object.entries(d).forEach(([id, diffsForToolCall]) => {
        const toApply = diffsForToolCall.map((diff) => {
          if (diff.toolCallId === id) return value;
          return diff.applied;
        });

        const chunks: DiffChunk[] = diffsForFile.map((diff) => diff.chunk);
        onSubmit(chunks, toApply, id);
      });
    },
    [onSubmit],
  );

  // TODO: Could be a single call to a mutation
  const applyAll = React.useCallback(() => {
    const groupedByToolCall = groupBy(values, (diff) => diff.toolCallId);
    Object.entries(groupedByToolCall).forEach(([toolCallId, diffs]) => {
      const toApply = diffs.map((diff) => !diff.applied);
      const chunks = [...diffs]
        .sort((a, b) => a.index - b.index)
        .reduce<DiffChunk[]>((acc, diff) => acc.concat(diff.chunk), []);
      onSubmit(chunks, toApply, toolCallId);
    });
  }, [onSubmit, values]);

  return (
    <Flex direction="column" maxWidth="100%" py="2" gap="2">
      {Object.entries(diffs).map(([fullFileName, diffsForFile], index) => {
        const key = fullFileName + "-" + index;

        // const errored = diffsForFile.some((diff) => diff.state === 2);
        const applied = diffsForFile.every((diff) => diff.applied);
        return (
          <Box key={key} my="2">
            <Flex justify="between" align="center" p="1">
              <TruncateLeft size="1">{fullFileName}</TruncateLeft>
              <Text size="1" as="label">
                <Flex align="center" gap="2" pl="2">
                  {/* {errored && "error"} */}
                  <Button
                    size="1"
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
                  <Diff
                    key={`${fullFileName}-${index}-${i}`}
                    diff={diff.chunk}
                  />
                ))}
              </Box>
            </ScrollArea>
          </Box>
        );
      })}

      <Flex gap="2" py="2">
        <Button disabled={!canApplyAll || loading} onClick={applyAll}>
          {action}
        </Button>
      </Flex>
    </Flex>
  );
};
