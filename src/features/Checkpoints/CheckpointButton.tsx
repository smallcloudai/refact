import { ResetIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";
import { useAppDispatch } from "../../hooks";
import {
  setIsCheckpointsPopupIsVisible,
  setLatestCheckpointResult,
} from "./checkpointsSlice";
import { useCallback } from "react";
import { Checkpoint } from "./types";
import { debugRefact } from "../../debugConfig";
import { useRestoreCheckpoints } from "../../hooks/useRestoreCheckpoints";

type CheckpointButtonProps = {
  checkpoints: Checkpoint[] | null;
};

export const CheckpointButton = ({ checkpoints }: CheckpointButtonProps) => {
  const dispatch = useAppDispatch();
  const { restoreChangesFromCheckpoints, isLoading } = useRestoreCheckpoints();

  const handleClick = useCallback(async () => {
    if (!checkpoints) return;

    debugRefact(
      `[DEBUG]: sending checkpoints (not implemented yet): `,
      checkpoints,
    );
    const restoredChanges =
      await restoreChangesFromCheckpoints(checkpoints).unwrap();
    debugRefact(`[DEBUG]: restoredChanges received: `, restoredChanges);
    const actions = [
      setLatestCheckpointResult(restoredChanges),
      setIsCheckpointsPopupIsVisible(true),
    ];
    actions.forEach((action) => dispatch(action));
  }, [dispatch, restoreChangesFromCheckpoints, checkpoints]);
  1;
  return (
    <IconButton
      size="2"
      variant="soft"
      title={isLoading ? "Reverting..." : "Revert agent changes"}
      onClick={() => void handleClick()}
      loading={isLoading}
    >
      <ResetIcon />
    </IconButton>
  );
};
