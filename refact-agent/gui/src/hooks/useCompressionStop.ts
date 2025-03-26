import { useEffect, useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import {
  selectLastSentCompression,
  selectThreadPaused,
  setThreadPaused,
} from "../features/Chat";

export function useLastSentCompressionStop() {
  const dispatch = useAppDispatch();
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  const stopped = useAppSelector(selectThreadPaused);
  useEffect(() => {
    if (lastSentCompression && lastSentCompression !== "absent") {
      dispatch(setThreadPaused(true));
    }
  }, [dispatch, lastSentCompression]);

  const resume = useCallback(() => {
    dispatch(setThreadPaused(false));
  }, [dispatch]);

  return { stopped, resume, strength: lastSentCompression };
}
