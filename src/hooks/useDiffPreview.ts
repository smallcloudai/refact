import { useCallback } from "react";
import { diffApi } from "../services/refact/diffs";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { DiffChunk } from "../events";

export const useDiffPreview = () => {
  const { diffPreview } = useEventsBusForIDE();

  const [submitPreview, result] = diffApi.useLazyDiffPreviewQuery();

  const onPreview = useCallback(
    async (chunks: DiffChunk[], toApply: boolean[]) => {
      const result = await submitPreview({ chunks, toApply });
      if (result.data) {
        diffPreview(result.data);
      }
    },
    [diffPreview, submitPreview],
  );

  return { onPreview, previewResult: result };
};
