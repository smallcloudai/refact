import { useCallback, useMemo } from "react";
import { v4 as uuidv4 } from "uuid";
import { useAppSelector } from "./useAppSelector";
import {
  chatDbMessageSliceActions,
  chatDbMessagesSliceSelectors,
} from "../features/ChatDB/chatDbMessagesSlice";
import {
  updateCMessagesThunk,
  updateThreadThunk,
} from "../services/refact/chatdb";
import { useAppDispatch } from "./useAppDispatch";
import { useSendChatRequest } from "./useSendChatRequest";
import {
  getSelectedSystemPrompt,
  selectThreadToolUse,
} from "../features/Chat/Thread/selectors";
import { CMessage, SystemMessage } from "../services/refact";
import { useGetCapsQuery } from "./useGetCapsQuery";
import { useGetPromptsQuery } from "./useGetPromptsQuery";

export function useThreadMessageSubmit() {
  const dispatch = useAppDispatch();
  const { maybeAddImagesToQuestion } = useSendChatRequest();
  const selectedSystemPrompt = useAppSelector(getSelectedSystemPrompt);
  const prompts = useGetPromptsQuery();
  const toolUse = useAppSelector(selectThreadToolUse);
  const caps = useGetCapsQuery();

  const thread = useAppSelector(chatDbMessagesSliceSelectors.selectThread);
  const leafPosition = useAppSelector(
    chatDbMessagesSliceSelectors.selectLeafEndPosition,
  );

  const systemMessageText = useMemo(() => {
    const defualtPropmpt = prompts.data?.default?.text ?? "";
    const selected = Object.values(selectedSystemPrompt);
    const prompt = selected.length > 0 ? selected[0].text : defualtPropmpt;
    return prompt;
  }, [prompts.data, selectedSystemPrompt]);

  const isNew = useMemo(() => {
    return !thread.cthread_id;
  }, [thread.cthread_id]);

  // TODO: use the hooks from crateApi for submitting threads and messages
  const submit = useCallback(
    async (question: string) => {
      const threadId = thread.cthread_id || uuidv4();
      const threadModel =
        (thread.cthread_model || caps.data?.code_chat_default_model) ?? "";
      const threadToolUse = (thread.cthread_toolset || toolUse) ?? "";
      const newThread = {
        ...thread,
        cthread_id: threadId,
        cthread_model: threadModel,
        cthread_toolset: threadToolUse,
      };

      const messagesToSend: CMessage[] = [];

      console.log({ thread, isNew });

      if (isNew) {
        const threadThunk = updateThreadThunk(newThread);
        await dispatch(threadThunk); // .unwrap(); // TODO: handle errors
        // this will subscribe to the thread's message list
        dispatch(chatDbMessageSliceActions.setThread(newThread));

        const systemMessage: SystemMessage = {
          role: "system",
          content: systemMessageText,
        };

        const systemCMessage: CMessage = {
          cmessage_belongs_to_cthread_id: threadId,
          cmessage_alt: 0,
          cmessage_num: 0,
          cmessage_prev_alt: -1,
          cmessage_usage_model: threadModel, // could be default
          cmessage_usage_prompt: 0,
          cmessage_usage_completion: 0,
          cmessage_json: systemMessage,
        };

        messagesToSend.push(systemCMessage);
      }

      const userMessage = maybeAddImagesToQuestion(question);
      const userCMessage: CMessage = {
        cmessage_belongs_to_cthread_id: threadId,
        cmessage_alt: leafPosition.alt,
        cmessage_num: leafPosition.num + 1,
        cmessage_prev_alt: leafPosition.alt, // TODO: add this to end tracker
        cmessage_usage_model: threadModel,
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: userMessage,
      };

      messagesToSend.push(userCMessage);

      console.log({ isNew, messagesToSend });

      const thunk = await dispatch(updateCMessagesThunk(messagesToSend));

      return thunk;
    },
    [
      caps.data?.code_chat_default_model,
      dispatch,
      isNew,
      leafPosition.alt,
      leafPosition.num,
      maybeAddImagesToQuestion,
      systemMessageText,
      thread,
      toolUse,
    ],
  );

  return { submit };

  // check if system message is needed,
  // then send the messages
}
