import { useCallback, useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { chatDbMessagesSliceSelectors } from "../features/ChatDB/chatDbMessagesSlice";
import { chatDbSelectors } from "../features/ChatDB/chatDbSlice";
import {
  sendMessagesThunk,
  updateThreadThunk,
} from "../services/refact/chatdb";
import { useAppDispatch } from "./useAppDispatch";
import { useSendChatRequest } from "./useSendChatRequest";
import { getSelectedSystemPrompt } from "../features/Chat/Thread/selectors";
import { CMessage, SystemMessage } from "../services/refact";

export function useThreadMessageSubmit() {
  const dispatch = useAppDispatch();
  const { maybeAddImagesToQuestion } = useSendChatRequest();
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);

  const thread = useAppSelector(chatDbMessagesSliceSelectors.selectThread);
  const leafPosition = useAppSelector(
    chatDbMessagesSliceSelectors.selectLeafEndPosition,
  );

  const maybeSavedThread = useAppSelector((state) =>
    chatDbSelectors.getThreadById(state, thread.cthread_id),
  );

  const isNew = useMemo(() => {
    return (
      !!maybeSavedThread && leafPosition.num === 0 && leafPosition.alt === 0
    );
  }, [leafPosition.alt, leafPosition.num, maybeSavedThread]);

  // TODO: use the hooks from crateApi for submitting threads and messages
  const submit = useCallback(
    async (question: string) => {
      if (isNew) {
        const threadThunk = updateThreadThunk(thread);
        await dispatch(threadThunk);
      }

      const messagesToSend: CMessage[] = [];

      if (
        isNew &&
        !("default" in systemPrompt) &&
        Object.values(systemPrompt).length > 0
      ) {
        const systemMessage: SystemMessage = {
          role: "system",
          content: Object.values(systemPrompt)[0].text,
        };

        const systemCMessage: CMessage = {
          cmessage_belongs_to_cthread_id: thread.cthread_id,
          cmessage_alt: 0,
          cmessage_num: -1,
          cmessage_prev_alt: 0,
          cmessage_usage_model: thread.cthread_model,
          cmessage_usage_prompt: 0,
          cmessage_usage_completion: 0,
          cmessage_json: systemMessage,
        };

        messagesToSend.push(systemCMessage);
      }

      // TODO: add system message

      const userMessage = maybeAddImagesToQuestion(question);
      const userCMessage: CMessage = {
        cmessage_belongs_to_cthread_id: thread.cthread_id,
        cmessage_alt: leafPosition.alt,
        cmessage_num: leafPosition.num,
        cmessage_prev_alt: leafPosition.alt, // TODO: add this
        cmessage_usage_model: thread.cthread_model,
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: userMessage,
      };

      messagesToSend.push(userCMessage);

      const thunk = await dispatch(
        sendMessagesThunk({ messages: messagesToSend }),
      );

      return thunk;
    },
    [
      dispatch,
      isNew,
      leafPosition.alt,
      leafPosition.num,
      maybeAddImagesToQuestion,
      systemPrompt,
      thread,
    ],
  );

  return { submit };

  // check if system message is needed,
  // then send the messages
}
