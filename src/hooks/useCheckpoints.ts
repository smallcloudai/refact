import { useCallback, useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import {
  selectIsCheckpointsPopupIsVisible,
  selectIsUndoingCheckpoints,
  setIsCheckpointsPopupIsVisible,
  setIsUndoingCheckpoints,
  setLatestCheckpointResult,
} from "../features/Checkpoints/checkpointsSlice";
import { useAppDispatch } from "./useAppDispatch";
import { useRestoreCheckpoints } from "./useRestoreCheckpoints";
import { Checkpoint } from "../features/Checkpoints/types";
import { debugRefact } from "../debugConfig";

export const useCheckpoints = () => {
  const dispatch = useAppDispatch();
  const { restoreChangesFromCheckpoints, isLoading } = useRestoreCheckpoints();
  const isCheckpointsPopupVisible = useAppSelector(
    selectIsCheckpointsPopupIsVisible,
  );
  const isUndoingCheckpoints = useAppSelector(selectIsUndoingCheckpoints);

  const shouldCheckpointsPopupBeShown = useMemo(() => {
    // TODO: show if isVisible and files data is not empty
    return isCheckpointsPopupVisible && !isUndoingCheckpoints;
  }, [isCheckpointsPopupVisible, isUndoingCheckpoints]);

  const handleUndo = useCallback(
    async (checkpoints: Checkpoint[]) => {
      await restoreChangesFromCheckpoints(checkpoints);
      dispatch(setIsUndoingCheckpoints(true));
    },
    [dispatch, restoreChangesFromCheckpoints],
  );

  const handleRestore = useCallback(
    async (checkpoints: Checkpoint[] | null) => {
      if (!checkpoints) return;
      dispatch(setIsUndoingCheckpoints(false));
      const restoredChanges =
        await restoreChangesFromCheckpoints(checkpoints).unwrap();
      debugRefact(`[DEBUG]: restoredChanges received: `, restoredChanges);
      const actions = [
        setLatestCheckpointResult(restoredChanges),
        setIsCheckpointsPopupIsVisible(true),
      ];
      actions.forEach((action) => dispatch(action));
    },
    [dispatch, restoreChangesFromCheckpoints],
  );

  return {
    shouldCheckpointsPopupBeShown,
    handleUndo,
    handleRestore,
    isLoading,
  };
};
