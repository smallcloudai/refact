import { useCallback, useEffect, useMemo } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  getSelectedSystemPrompt,
  selectAutomaticPatch,
  selectChatError,
  selectChatId,
  selectCheckpointsEnabled,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectPreventSend,
  selectSendImmediately,
  selectThread,
  selectThreadMode,
  selectThreadToolUse,
} from "../features/Chat/Thread/selectors";
import {
  useCheckForConfirmationMutation,
  // useGetToolsLazyQuery,
} from "./useGetToolGroupsQuery";
import {
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
  isUserMessage,
  UserMessage,
  UserMessageContentWithImage,
} from "../services/refact/types";
import {
  backUpMessages,
  chatAskQuestionThunk,
  chatAskedQuestion,
  setSendImmediately,
} from "../features/Chat/Thread/actions";

import { selectAllImages } from "../features/AttachedImages";
import { useAbortControllers } from "./useAbortControllers";
import {
  clearPauseReasonsAndHandleToolsStatus,
  getToolsConfirmationStatus,
  getToolsInteractionStatus,
  resetConfirmationInteractedState,
  setPauseReasons,
} from "../features/ToolConfirmation/confirmationSlice";
import {
  chatModeToLspMode,
  LspChatMode,
  setChatMode,
  setIsWaitingForResponse,
  setLastUserMessageId,
  upsertToolCall,
} from "../features/Chat";

import { v4 as uuidv4 } from "uuid";
import { upsertToolCallIntoHistory } from "../features/History/historySlice";

type SubmitHandlerParams =
  | {
      question: string;
      maybeMode?: LspChatMode;
      maybeMessages?: undefined;
      maybeDropLastMessage?: boolean;
    }
  | {
      question?: undefined;
      maybeMode?: LspChatMode;
      maybeMessages?: undefined;
      maybeDropLastMessage?: boolean;
    }
  | {
      question?: undefined;
      maybeMode?: LspChatMode;
      maybeMessages: ChatMessage[];
      maybeDropLastMessage?: boolean;
    };

let recallCounter = 0;

export const PATCH_LIKE_FUNCTIONS = [
  "patch",
  "text_edit",
  "create_textdoc",
  "update_textdoc",
  "replace_textdoc",
  "update_textdoc_regex",
];

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const abortControllers = useAbortControllers();

  // const [triggerGetTools] = useGetToolsLazyQuery();
  const [triggerCheckForConfirmation] = useCheckForConfirmationMutation();

  const chatId = useAppSelector(selectChatId);

  const isWaiting = useAppSelector(selectIsWaiting);

  const currentMessages = useAppSelector(selectMessages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);
  const toolUse = useAppSelector(selectThreadToolUse);
  const attachedImages = useAppSelector(selectAllImages);
  const threadMode = useAppSelector(selectThreadMode);
  const threadIntegration = useAppSelector(selectIntegration);
  const wasInteracted = useAppSelector(getToolsInteractionStatus); // shows if tool confirmation popup was interacted by user
  const areToolsConfirmed = useAppSelector(getToolsConfirmationStatus);

  const isPatchAutomatic = useAppSelector(selectAutomaticPatch);
  const checkpointsEnabled = useAppSelector(selectCheckpointsEnabled);

  const messagesWithSystemPrompt = useMemo(() => {
    const prompts = Object.entries(systemPrompt);
    if (prompts.length === 0) return currentMessages;
    const [key, prompt] = prompts[0];
    if (key === "default") return currentMessages;
    if (currentMessages.length === 0) {
      const message: ChatMessage = { role: "system", content: prompt.text };
      return [message];
    }
    return currentMessages;
  }, [currentMessages, systemPrompt]);

  const sendMessages = useCallback(
    async (messages: ChatMessages, maybeMode?: LspChatMode) => {
      dispatch(setIsWaitingForResponse(true));
      // TODO: should be safe to remove

      // let tools = await triggerGetTools(undefined).unwrap();
      // // TODO: save tool use to state.chat
      // // if (toolUse && isToolUse(toolUse)) {
      // //   dispatch(setToolUse(toolUse));
      // // }
      // if (toolUse === "quick") {
      //   tools = [];
      // } else if (toolUse === "explore") {
      //   tools = tools.filter((t) => !t.function.agentic);
      // }
      // tools = tools.map((t) => {
      //   const { agentic: _, ...remaining } = t.function;
      //   return { ...t, function: { ...remaining } };
      // });

      const lastMessage = messages.slice(-1)[0];

      if (
        !isWaiting &&
        !wasInteracted &&
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls
      ) {
        const toolCalls = lastMessage.tool_calls;
        if (
          !(
            toolCalls[0].function.name &&
            PATCH_LIKE_FUNCTIONS.includes(toolCalls[0].function.name) &&
            isPatchAutomatic
          )
        ) {
          const confirmationResponse = await triggerCheckForConfirmation({
            tool_calls: toolCalls,
            messages: messages,
          }).unwrap();
          if (confirmationResponse.pause) {
            dispatch(setPauseReasons(confirmationResponse.pause_reasons));
            return;
          }
        }
      }

      dispatch(backUpMessages({ id: chatId, messages }));
      dispatch(chatAskedQuestion({ id: chatId }));

      const mode =
        maybeMode ?? chatModeToLspMode({ toolUse, mode: threadMode });

      const maybeLastUserMessageIsFromUser = isUserMessage(lastMessage);
      if (maybeLastUserMessageIsFromUser) {
        dispatch(setLastUserMessageId({ chatId: chatId, messageId: uuidv4() }));
      }

      const action = chatAskQuestionThunk({
        messages,
        // tools
        checkpointsEnabled,
        chatId,
        mode,
      });

      const dispatchedAction = dispatch(action);
      abortControllers.addAbortController(chatId, dispatchedAction.abort);
    },
    [
      // triggerGetTools,
      toolUse,
      isWaiting,
      dispatch,
      chatId,
      threadMode,
      wasInteracted,
      checkpointsEnabled,
      abortControllers,
      triggerCheckForConfirmation,
      isPatchAutomatic,
    ],
  );

  const maybeAddImagesToQuestion = useCallback(
    (question: string): UserMessage => {
      if (attachedImages.length === 0)
        return { role: "user" as const, content: question, checkpoints: [] };

      const images = attachedImages.reduce<UserMessageContentWithImage[]>(
        (acc, image) => {
          if (typeof image.content !== "string") return acc;
          return acc.concat({
            type: "image_url",
            image_url: { url: image.content },
          });
        },
        [],
      );

      if (images.length === 0)
        return { role: "user", content: question, checkpoints: [] };

      return {
        role: "user",
        content: [...images, { type: "text", text: question }],
        checkpoints: [],
      };
    },
    [attachedImages],
  );

  const submit = useCallback(
    ({
      question,
      maybeMode,
      maybeMessages,
      maybeDropLastMessage,
    }: SubmitHandlerParams) => {
      let messages = messagesWithSystemPrompt;
      if (maybeDropLastMessage) {
        messages = messages.slice(0, -1);
      }

      if (question) {
        const message = maybeAddImagesToQuestion(question);
        messages = messages.concat(message);
      } else if (maybeMessages) {
        messages = maybeMessages;
      }

      // TODO: make a better way for setting / detecting thread mode.
      const maybeConfigure = threadIntegration ? "CONFIGURE" : undefined;
      const mode = chatModeToLspMode({
        toolUse,
        mode: maybeMode ?? threadMode ?? maybeConfigure,
      });
      dispatch(setChatMode(mode));

      void sendMessages(messages, mode);
    },
    [
      dispatch,
      maybeAddImagesToQuestion,
      messagesWithSystemPrompt,
      sendMessages,
      threadIntegration,
      threadMode,
      toolUse,
    ],
  );

  const abort = useCallback(() => {
    abortControllers.abort(chatId);
  }, [abortControllers, chatId]);

  const retry = useCallback(
    (messages: ChatMessages) => {
      abort();
      dispatch(
        clearPauseReasonsAndHandleToolsStatus({
          wasInteracted: false,
          confirmationStatus: areToolsConfirmed,
        }),
      );
      void sendMessages(messages);
    },
    [abort, sendMessages, dispatch, areToolsConfirmed],
  );

  const confirmToolUsage = useCallback(() => {
    abort();
    dispatch(
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: true,
        confirmationStatus: true,
      }),
    );

    dispatch(setIsWaitingForResponse(false));
  }, [abort, dispatch]);

  const rejectToolUsage = useCallback(
    (toolCallIds: string[]) => {
      abort();

      toolCallIds.forEach((toolCallId) => {
        dispatch(
          upsertToolCallIntoHistory({ toolCallId, chatId, accepted: false }),
        );
        dispatch(upsertToolCall({ toolCallId, chatId, accepted: false }));
      });

      dispatch(resetConfirmationInteractedState());
      dispatch(setIsWaitingForResponse(false));
    },
    [abort, chatId, dispatch],
  );

  const retryFromIndex = useCallback(
    (index: number, question: UserMessage["content"]) => {
      const messagesToKeep = currentMessages.slice(0, index);
      const messagesToSend = messagesToKeep.concat([
        { role: "user", content: question, checkpoints: [] },
      ]);
      retry(messagesToSend);
    },
    [currentMessages, retry],
  );

  return {
    submit,
    abort,
    retry,
    retryFromIndex,
    confirmToolUsage,
    maybeAddImagesToQuestion,
    rejectToolUsage,
    sendMessages,
    messagesWithSystemPrompt,
  };
};

// NOTE: only use this once
export function useAutoSend() {
  const dispatch = useAppDispatch();
  const streaming = useAppSelector(selectIsStreaming);
  const currentMessages = useAppSelector(selectMessages);
  const errored = useAppSelector(selectChatError);
  const preventSend = useAppSelector(selectPreventSend);
  const isWaiting = useAppSelector(selectIsWaiting);
  const sendImmediately = useAppSelector(selectSendImmediately);
  const wasInteracted = useAppSelector(getToolsInteractionStatus); // shows if tool confirmation popup was interacted by user
  const areToolsConfirmed = useAppSelector(getToolsConfirmationStatus);
  const { sendMessages, abort, messagesWithSystemPrompt } =
    useSendChatRequest();
  // TODO: make a selector for this, or show tool formation
  const thread = useAppSelector(selectThread);
  const isIntegration = thread.integration ?? false;

  useEffect(() => {
    if (sendImmediately) {
      dispatch(setSendImmediately(false));
      void sendMessages(messagesWithSystemPrompt);
    }
  }, [dispatch, messagesWithSystemPrompt, sendImmediately, sendMessages]);

  useEffect(() => {
    if (
      !isWaiting &&
      !streaming &&
      currentMessages.length > 0 &&
      !errored &&
      !preventSend
    ) {
      const lastMessage = currentMessages.slice(-1)[0];

      if (
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls &&
        lastMessage.tool_calls.length > 0
      ) {
        if (!isIntegration && !wasInteracted && !areToolsConfirmed) {
          abort();
          if (recallCounter < 1) {
            recallCounter++;
            return;
          }
        }

        dispatch(
          clearPauseReasonsAndHandleToolsStatus({
            wasInteracted: false,
            confirmationStatus: areToolsConfirmed,
          }),
        );
        void sendMessages(currentMessages, thread.mode);
        recallCounter = 0;
      }
    }
  }, [
    dispatch,
    errored,
    currentMessages,
    preventSend,
    sendMessages,
    abort,
    streaming,
    wasInteracted,
    areToolsConfirmed,
    isWaiting,
    isIntegration,
    thread.mode,
    thread,
  ]);
}
