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

  const { handlePreview, isPreviewing } = useCheckpoints();

  return (
    <IconButton
      size="2"
      variant="soft"
      title={isPreviewing ? "Reverting..." : "Revert agent changes"}
      onClick={() => void handlePreview(checkpoints, messageIndex)}
      loading={isPreviewing}
      disabled={!isOnline || isStreaming || isWaiting}
    >
      <ResetIcon />
    </IconButton>
  );
};
