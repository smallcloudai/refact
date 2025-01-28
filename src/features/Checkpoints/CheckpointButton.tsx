import { ResetIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";
import { Checkpoint } from "./types";
import { useCheckpoints } from "../../hooks/useCheckpoints";

type CheckpointButtonProps = {
  checkpoints: Checkpoint[] | null;
  messageIndex: number;
};

export const CheckpointButton = ({
  checkpoints,
  messageIndex,
}: CheckpointButtonProps) => {
  const { handleRestore, isLoading } = useCheckpoints();
  return (
    <IconButton
      size="2"
      variant="soft"
      title={isLoading ? "Reverting..." : "Revert agent changes"}
      onClick={() => void handleRestore(checkpoints, messageIndex)}
      loading={isLoading}
    >
      <ResetIcon />
    </IconButton>
  );
};
