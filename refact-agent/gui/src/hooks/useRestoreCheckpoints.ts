import { useCallback } from "react";
import { Checkpoint } from "../features/Checkpoints/types";
import { checkpointsApi } from "../services/refact/checkpoints";

export const useRestoreCheckpoints = () => {
  const [mutationTrigger, { isLoading }] =
    checkpointsApi.useRestoreCheckpointsMutation();

  const restoreChangesFromCheckpoints = useCallback(
    (checkpoints: Checkpoint[]) => {
      return mutationTrigger({ checkpoints });
    },
    [mutationTrigger],
  );

  return { restoreChangesFromCheckpoints, isLoading };
};
