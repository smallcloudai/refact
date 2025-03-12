import { useCallback, useEffect, useMemo } from "react";
import { useCapsForToolUse } from "./useCapsForToolUse";
import { useAppSelector } from "./useAppSelector";
import {
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectThreadBoostReasoning,
  setBoostReasoning,
} from "../features/Chat";
import { useAppDispatch } from "./useAppDispatch";

export function useThinking() {
  const dispatch = useAppDispatch();

  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const chatId = useAppSelector(selectChatId);

  const isBoostReasoningEnabled = useAppSelector(selectThreadBoostReasoning);

  const caps = useCapsForToolUse();

  const supportsBoostReasoning = useMemo(() => {
    const models = caps.data?.code_chat_models;
    const item = models?.[caps.currentModel];
    return item?.supports_boost_reasoning ?? false;
  }, [caps.data?.code_chat_models, caps.currentModel]);

  const shouldBeDisabled = useMemo(() => {
    return !supportsBoostReasoning || isStreaming || isWaiting;
  }, [supportsBoostReasoning, isStreaming, isWaiting]);

  const noteText = useMemo(() => {
    if (!supportsBoostReasoning)
      return `Note: ${caps.currentModel} doesn't support thinking`;
    if (isStreaming || isWaiting)
      return `Note: you can't ${
        isBoostReasoningEnabled ? "disable" : "enable"
      } reasoning while stream is in process`;
  }, [
    supportsBoostReasoning,
    isStreaming,
    isWaiting,
    isBoostReasoningEnabled,
    caps.currentModel,
  ]);

  const handleReasoningChange = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>, checked: boolean) => {
      event.stopPropagation();
      event.preventDefault();
      dispatch(setBoostReasoning({ chatId, value: checked }));
    },
    [dispatch, chatId],
  );

  useEffect(() => {
    if (!supportsBoostReasoning) {
      dispatch(setBoostReasoning({ chatId, value: supportsBoostReasoning }));
    }
  }, [dispatch, chatId, supportsBoostReasoning, shouldBeDisabled]);

  return {
    handleReasoningChange,
    shouldBeDisabled,
    noteText,
    areCapsInitialized: !caps.uninitialized,
  };
}
