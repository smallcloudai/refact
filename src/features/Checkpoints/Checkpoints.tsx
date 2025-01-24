import { Dialog, Flex, Text, Button, ScrollArea } from "@radix-ui/themes";
import { useCallback, useState } from "react";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { useAppDispatch } from "../../hooks";
import { setIsCheckpointsPopupIsVisible } from "./checkpointsSlice";
import { Markdown } from "../../components/Markdown";
import {
  FileChanged,
  FileChangedStatus,
  RestoreCheckpointsResponse,
  RevertedCheckpointData,
} from "./types";
import { TruncateLeft } from "../../components/Text";
import { Link } from "../../components/Link";

interface CheckpointFile {
  absolute_ppath: string;
  status: "success" | "warning" | "error";
}

interface CheckpointProps {
  hash: string;
  files: FileChanged[];
  onFix?: () => void;
  onUndo?: () => void;
}

const StatusIndicator = ({ status }: { status: FileChangedStatus }) => {
  const colors = {
    A: "#22C55E",
    M: "#F59E0B",
    D: "#EF4444",
  };

  return (
    <Text
      size="1"
      style={{
        color: colors[status],
        flexGrow: 1,
      }}
    >
      {status}
    </Text>
  );
};

export const Checkpoints = ({
  hash,
  files,
  onFix,
  onUndo,
}: CheckpointProps) => {
  const dispatch = useAppDispatch();
  const { shouldCheckpointsPopupBeShown } = useCheckpoints();
  const [open, setOpen] = useState(shouldCheckpointsPopupBeShown);

  const handleOpenChange = useCallback(
    (value: boolean) => {
      setOpen(value);
      dispatch(setIsCheckpointsPopupIsVisible(value));
    },
    [dispatch],
  );

  return (
    <Dialog.Root open={open} onOpenChange={handleOpenChange}>
      <Dialog.Content style={{ maxWidth: "500px" }}>
        <Dialog.Title as="h3" mb="3">
          <Markdown color="indigo">
            {"Checkpoint " + "```" + hash + "```"}
          </Markdown>
        </Dialog.Title>

        <ScrollArea style={{ maxHeight: "300px" }}>
          <Flex direction="column" gap="2">
            {files.map((file, index) => (
              <Flex
                key={index}
                align="center"
                justify="between"
                style={{
                  padding: "8px",
                  backgroundColor: "var(--gray-3)",
                  borderRadius: "var(--radius-3)",
                }}
                width="100%"
              >
                {/* <TruncateLeft size="1">
                  <Text
                    size="2"
                    style={{
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      flex: 1,
                      textDecoration:
                        file.status === "D" ? "line-through" : undefined,
                    }}
                  >
                    {file.absolute_path}
                  </Text>
                </TruncateLeft> */}
                <TruncateLeft>
                  <Link size="2" color="gray">
                    tests/emergency_frog_situation/frog.py
                  </Link>
                </TruncateLeft>
                <StatusIndicator status={file.status} />
              </Flex>
            ))}
          </Flex>
        </ScrollArea>

        <Flex gap="3" mt="4" justify="end">
          <Button variant="soft" color="gray" onClick={() => onUndo?.()}>
            Undo
          </Button>
          <Button onClick={() => onFix?.()}>Fix</Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};
