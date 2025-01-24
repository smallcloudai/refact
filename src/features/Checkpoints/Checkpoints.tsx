import { Dialog, Flex, Text, Button } from "@radix-ui/themes";
import { useCallback, useState } from "react";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { useAppDispatch, useEventsBusForIDE } from "../../hooks";
import { setIsCheckpointsPopupIsVisible } from "./checkpointsSlice";
import { Markdown } from "../../components/Markdown";
import { FileChanged, FileChangedStatus } from "./types";
import { TruncateLeft } from "../../components/Text";
import { Link } from "../../components/Link";
import { ScrollArea } from "../../components/ScrollArea";

import styles from "./Checkpoints.module.css";

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
  const { openFile } = useEventsBusForIDE();
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
      <Dialog.Content className={styles.CheckpointsDialog}>
        <Dialog.Title as="h3" mb="3">
          <Markdown color="indigo">
            {"Checkpoint " + "```" + hash + "```"}
          </Markdown>
        </Dialog.Title>

        <ScrollArea scrollbars="vertical" style={{ maxHeight: "300px" }}>
          <Flex direction="column" gap="2">
            {files.map((file, index) => (
              <Flex
                key={index}
                gap="2"
                py="2"
                px="2"
                justify="between"
                align="center"
                style={{
                  backgroundColor: "var(--gray-3)",
                  borderRadius: "var(--radius-3)",
                }}
              >
                <TruncateLeft size="2">
                  <Link
                    title="Open file"
                    onClick={(event) => {
                      event.preventDefault();
                      openFile({ file_name: file.absolute_path });
                    }}
                  >
                    {file.absolute_path}
                  </Link>
                </TruncateLeft>{" "}
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
