import { useCallback } from "react";
import { diffApi } from "../services/refact/diffs";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { DiffChunk } from "../events";

export const useDiffPreview = (chunks: DiffChunk[]) => {
  const { diffPreview } = useEventsBusForIDE();

  const [submitPreview, result] = diffApi.useLazyDiffPreviewQuery();

  const onPreview = useCallback(
    async (toApply: boolean[]) => {
      const result = await submitPreview({ chunks, toApply });
      if (result.data) {
        diffPreview(result.data);
      }
    },
    [chunks, diffPreview, submitPreview],
  );

  return { onPreview, result };
};
