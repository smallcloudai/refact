import { ResetIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";
import { Checkpoint } from "./types";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { useAppSelector, useIsOnline } from "../../hooks";
import { selectIsStreaming, selectIsWaiting } from "../Chat";

type CheckpointButtonProps = {
  checkpoints: Checkpoint[] | null;
  messageIndex: number;
};

export const CheckpointButton = ({
  checkpoints,
  messageIndex,
}: CheckpointButtonProps) => {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const isOnline = useIsOnline();

  const { handleRestore, isLoading } = useCheckpoints();

  return (
    <IconButton
      size="2"
      variant="soft"
      title={isLoading ? "Reverting..." : "Revert agent changes"}
      onClick={() => void handleRestore(checkpoints, messageIndex)}
      loading={isLoading}
      disabled={!isOnline || isStreaming || isWaiting}
    >
      <ResetIcon />
    </IconButton>
  );
};
