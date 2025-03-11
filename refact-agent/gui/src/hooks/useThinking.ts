import { useCallback, useEffect, useMemo } from "react";
import { useCapsForToolUse } from "./useCapsForToolUse";
import { useAppSelector } from "./useAppSelector";
import {
  selectChatId,
  selectMessages,
  setBoostReasoning,
} from "../features/Chat";
import { useAppDispatch } from "./useAppDispatch";

export function useThinking() {
  const dispatch = useAppDispatch();

  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);

  const caps = useCapsForToolUse();

  const supportsBoostReasoning = useMemo(() => {
    const models = caps.data?.code_chat_models;
    const item = models?.[caps.currentModel];
    return item?.supports_boost_reasoning ?? false;
  }, [caps.data?.code_chat_models, caps.currentModel]);

  const shouldBeDisabled = useMemo(() => {
    return !supportsBoostReasoning || messages.length > 0;
  }, [supportsBoostReasoning, messages]);

  const handleReasoningChange = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>, checked: boolean) => {
      event.stopPropagation();
      event.preventDefault();
      dispatch(setBoostReasoning({ chatId, value: checked }));
    },
    [dispatch, chatId],
  );

  useEffect(() => {
    if (shouldBeDisabled) {
      dispatch(setBoostReasoning({ chatId, value: supportsBoostReasoning }));
    }
  }, [dispatch, chatId, supportsBoostReasoning, shouldBeDisabled]);

  return {
    handleReasoningChange,
    shouldBeDisabled,
    supportsBoostReasoning,
    currentModelFromCaps: caps.currentModel,
  };
}
