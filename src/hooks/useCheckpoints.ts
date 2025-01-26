import { useCallback, useMemo, useState } from "react";
import { useAppSelector } from "./useAppSelector";
import {
  selectIsCheckpointsPopupIsVisible,
  selectIsUndoingCheckpoints,
  selectLatestCheckpointResult,
  setIsCheckpointsPopupIsVisible,
  setIsUndoingCheckpoints,
  setLatestCheckpointResult,
} from "../features/Checkpoints/checkpointsSlice";
import { useAppDispatch } from "./useAppDispatch";
import { useRestoreCheckpoints } from "./useRestoreCheckpoints";
import { Checkpoint, FileChanged } from "../features/Checkpoints/types";
import { debugRefact } from "../debugConfig";
import { STUB_RESTORED_CHECKPOINT_DATA } from "../__fixtures__/checkpoints";

export const useCheckpoints = () => {
  const dispatch = useAppDispatch();
  const { restoreChangesFromCheckpoints, isLoading } = useRestoreCheckpoints();
  const isCheckpointsPopupVisible = useAppSelector(
    selectIsCheckpointsPopupIsVisible,
  );
  const isUndoingCheckpoints = useAppSelector(selectIsUndoingCheckpoints);

  const latestRestoredCheckpointsResult = useAppSelector(
    selectLatestCheckpointResult,
  );

  const [shouldMockBeUsed, setShouldMockBeUsed] = useState(false);

  const { reverted_changes, reverted_to } = shouldMockBeUsed
    ? STUB_RESTORED_CHECKPOINT_DATA
    : latestRestoredCheckpointsResult;

  const allChangedFiles = reverted_changes.reduce<
    (FileChanged & { workspace_folder: string })[]
  >((acc, change) => {
    const filesWithWorkspace = change.files_changed.map((file) => ({
      ...file,
      workspace_folder: change.workspace_folder,
    }));
    return [...acc, ...filesWithWorkspace];
  }, []);

  const wereFilesChanged = useMemo(() => {
    return allChangedFiles.length > 0;
  }, [allChangedFiles]);

  const shouldCheckpointsPopupBeShown = useMemo(() => {
    return isCheckpointsPopupVisible && !isUndoingCheckpoints;
  }, [isCheckpointsPopupVisible, isUndoingCheckpoints]);

  const handleUndo = useCallback(async () => {
    await restoreChangesFromCheckpoints(
      latestRestoredCheckpointsResult.checkpoints_for_undo,
    );
    dispatch(setIsUndoingCheckpoints(true));
  }, [
    dispatch,
    restoreChangesFromCheckpoints,
    latestRestoredCheckpointsResult.checkpoints_for_undo,
  ]);

  const handleRestore = useCallback(
    async (checkpoints: Checkpoint[] | null) => {
      if (!checkpoints) return;
      const restoredChanges =
        await restoreChangesFromCheckpoints(checkpoints).unwrap();
      debugRefact(`[DEBUG]: restoredChanges received: `, restoredChanges);
      const actions = [
        dispatch(setIsUndoingCheckpoints(false)),
        setLatestCheckpointResult(restoredChanges),
        setIsCheckpointsPopupIsVisible(true),
      ];
      actions.forEach((action) => dispatch(action));
    },
    [dispatch, restoreChangesFromCheckpoints],
  );

  const handleFix = useCallback(() => {
    dispatch(setIsCheckpointsPopupIsVisible(false));
  }, [dispatch]);

  // TODO: remove when fully tested
  const handleShouldMockBeUsedChange = useCallback(() => {
    setShouldMockBeUsed((prev) => !prev);
  }, []);
  // end TODO

  return {
    shouldCheckpointsPopupBeShown,
    handleUndo,
    handleRestore,
    handleFix,
    // TODO: remove when fully tested
    handleShouldMockBeUsedChange,
    shouldMockBeUsed,
    // end TODO
    isLoading,
    reverted_changes,
    reverted_to,
    wereFilesChanged,
    allChangedFiles,
  };
};
