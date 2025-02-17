import { useEffect } from "react";
import {
  removePatchMetaByFileNameIfCompleted,
  selectCompletedPatchesFilePaths,
  selectUnsentPatchesFilePaths,
  setStartedByFilePaths,
} from "../features/PatchesAndDiffsTracker/patchesAndDiffsTrackerSlice";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import { useEventsBusForIDE } from "./useEventBusForIDE";

export function usePatchesAndDiffsEventsForIDE() {
  const dispatch = useAppDispatch();
  const unsent = useAppSelector(selectUnsentPatchesFilePaths);
  const completed = useAppSelector(selectCompletedPatchesFilePaths);
  const { startFileAnimation, stopFileAnimation } = useEventsBusForIDE();

  useEffect(() => {
    if (!unsent.length) return;
    unsent.forEach((filePath) => {
      startFileAnimation(filePath);
    });

    dispatch(setStartedByFilePaths(unsent));
  }, [dispatch, startFileAnimation, unsent]);

  useEffect(() => {
    if (!completed.length) return;
    completed.forEach((filePath) => {
      stopFileAnimation(filePath);
    });
    dispatch(removePatchMetaByFileNameIfCompleted(completed));
  }, [dispatch, completed, stopFileAnimation]);
}
