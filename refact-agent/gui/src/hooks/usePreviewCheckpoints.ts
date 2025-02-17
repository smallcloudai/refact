import { useCallback } from "react";
import { Checkpoint } from "../features/Checkpoints/types";
import { checkpointsApi } from "../services/refact/checkpoints";

export const usePreviewCheckpoints = () => {
  const [mutationTrigger, { isLoading }] =
    checkpointsApi.usePreviewCheckpointsMutation();

  const previewChangesFromCheckpoints = useCallback(
    (checkpoints: Checkpoint[]) => {
      return mutationTrigger({ checkpoints });
    },
    [mutationTrigger],
  );

  return { previewChangesFromCheckpoints, isLoading };
};
