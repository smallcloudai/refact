import { ResetIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";
import { Checkpoint } from "./types";
import { useCheckpoints } from "../../hooks/useCheckpoints";

type CheckpointButtonProps = {
  checkpoints: Checkpoint[] | null;
};

export const CheckpointButton = ({ checkpoints }: CheckpointButtonProps) => {
  const { handleRestore, isLoading } = useCheckpoints();
  return (
    <IconButton
      size="2"
      variant="soft"
      title={isLoading ? "Reverting..." : "Revert agent changes"}
      onClick={() => void handleRestore(checkpoints)}
      loading={isLoading}
    >
      <ResetIcon />
    </IconButton>
  );
};
