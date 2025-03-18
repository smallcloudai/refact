import { useCallback } from "react";
import { selectThread } from "../features/Chat/Thread/selectors";
import { useAppSelector } from "./useAppSelector";
import { knowledgeApi } from "../services/refact";
import { newChatAction } from "../events";
import { useAppDispatch } from "./useAppDispatch";
import { setError } from "../features/Errors/errorsSlice";
import { setIsWaitingForResponse } from "../features/Chat";

export function useCompressChat() {
  const dispatch = useAppDispatch();
  const thread = useAppSelector(selectThread);

  const [submit, request] = knowledgeApi.useCompressMessagesMutation();

  const compressChat = useCallback(async () => {
    dispatch(setIsWaitingForResponse(true));
    const result = await submit({
      messages: thread.messages,
      project: thread.project_name ?? "",
    });
    dispatch(setIsWaitingForResponse(false));

    if (result.error) {
      // TODO: handle errors
      dispatch(
        setError("Error compressing chat: " + JSON.stringify(result.error)),
      );
    }

    // TODO: create new chat with result.data as user message.
    if (result.data) {
      const newThreadAction = newChatAction({
        messages: [{ role: "user", content: result.data.trajectory }],
      });
      dispatch(newThreadAction);
    }
  }, [submit, thread.messages, thread.project_name, dispatch]);

  return { compressChat, compressChatRequest: request };
}
