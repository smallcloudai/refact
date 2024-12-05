import { useCallback, useEffect, useMemo } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  getSelectedSystemPrompt,
  selectChatError,
  selectChatId,
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
  useGetToolsLazyQuery,
} from "./useGetToolsQuery";
import {
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
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
  clearPauseReasonsAndConfirmTools,
  getToolsConfirmationStatus,
  setPauseReasons,
} from "../features/ToolConfirmation/confirmationSlice";
import { chatModeToLspMode, LspChatMode, setChatMode } from "../features/Chat";

let recallCounter = 0;

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const abortControllers = useAbortControllers();

  const [triggerGetTools] = useGetToolsLazyQuery();
  const [triggerCheckForConfirmation] = useCheckForConfirmationMutation();

  const chatId = useAppSelector(selectChatId);

  const isWaiting = useAppSelector(selectIsWaiting);

  const currentMessages = useAppSelector(selectMessages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);
  const sendImmediately = useAppSelector(selectSendImmediately);
  const toolUse = useAppSelector(selectThreadToolUse);
  const attachedImages = useAppSelector(selectAllImages);
  const threadMode = useAppSelector(selectThreadMode);
  const threadIntegration = useAppSelector(selectIntegration);
  const areToolsConfirmed = useAppSelector(getToolsConfirmationStatus);

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
    async (messages: ChatMessages) => {
      let tools = await triggerGetTools(undefined).unwrap();
      // TODO: save tool use to state.chat
      // if (toolUse && isToolUse(toolUse)) {
      //   dispatch(setToolUse(toolUse));
      // }
      if (toolUse === "quick") {
        tools = [];
      } else if (toolUse === "explore") {
        tools = tools.filter((t) => !t.function.agentic);
      }
      tools = tools.map((t) => {
        const { agentic: _, ...remaining } = t.function;
        return { ...t, function: { ...remaining } };
      });

      const lastMessage = messages.slice(-1)[0];
      if (
        !isWaiting &&
        !areToolsConfirmed &&
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls
      ) {
        const toolCalls = lastMessage.tool_calls;
        const confirmationResponse =
          await triggerCheckForConfirmation(toolCalls).unwrap();
        if (confirmationResponse.pause) {
          dispatch(setPauseReasons(confirmationResponse.pause_reasons));
          return;
        }
      }

      dispatch(backUpMessages({ id: chatId, messages }));
      dispatch(chatAskedQuestion({ id: chatId }));

      // TODO: this is calculated twice.
      const mode = chatModeToLspMode(toolUse, threadMode);

      const action = chatAskQuestionThunk({
        messages,
        tools,
        chatId,
        mode,
      });

      const dispatchedAction = dispatch(action);
      abortControllers.addAbortController(chatId, dispatchedAction.abort);
    },
    [
      triggerGetTools,
      toolUse,
      isWaiting,
      areToolsConfirmed,
      dispatch,
      chatId,
      threadMode,
      abortControllers,
      triggerCheckForConfirmation,
    ],
  );

  const maybeAddImagesToQuestion = useCallback(
    (question: string): UserMessage => {
      if (attachedImages.length === 0)
        return { role: "user" as const, content: question };

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

      if (images.length === 0) return { role: "user", content: question };

      return {
        role: "user",
        content: [...images, { type: "text", text: question }],
      };
    },
    [attachedImages],
  );

  const submit = useCallback(
    (question: string, maybeMode?: LspChatMode) => {
      // const message: ChatMessage = { role: "user", content: question };
      const message: UserMessage = maybeAddImagesToQuestion(question);
      const messages = messagesWithSystemPrompt.concat(message);
      const maybeConfigure = threadIntegration ? "CONFIGURE" : undefined;
      // Save the mode
      const mode = chatModeToLspMode(toolUse, maybeMode ?? maybeConfigure);
      dispatch(setChatMode(mode));

      void sendMessages(messages);
    },
    [
      dispatch,
      maybeAddImagesToQuestion,
      messagesWithSystemPrompt,
      sendMessages,
      threadIntegration,
      toolUse,
    ],
  );

  const abort = useCallback(() => {
    abortControllers.abort(chatId);
  }, [abortControllers, chatId]);

  useEffect(() => {
    if (sendImmediately) {
      dispatch(setSendImmediately(false));
      void sendMessages(messagesWithSystemPrompt);
    }
  }, [dispatch, messagesWithSystemPrompt, sendImmediately, sendMessages]);

  const retry = useCallback(
    (messages: ChatMessages) => {
      abort();
      dispatch(clearPauseReasonsAndConfirmTools(false));
      void sendMessages(messages);
    },
    [abort, sendMessages, dispatch],
  );

  const confirmToolUsage = useCallback(() => {
    abort();
    dispatch(clearPauseReasonsAndConfirmTools(true));
  }, [abort, dispatch]);

  const retryFromIndex = useCallback(
    (index: number, question: UserMessage["content"]) => {
      const messagesToKeep = currentMessages.slice(0, index);
      const messagesToSend = messagesToKeep.concat([
        { role: "user", content: question },
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
    sendMessages,
  };
};

// NOTE: only use this once
export function useAutoSend() {
  const streaming = useAppSelector(selectIsStreaming);
  const currentMessages = useAppSelector(selectMessages);
  const errored = useAppSelector(selectChatError);
  const preventSend = useAppSelector(selectPreventSend);
  const isWaiting = useAppSelector(selectIsWaiting);
  const areToolsConfirmed = useAppSelector(getToolsConfirmationStatus);
  const { sendMessages, abort } = useSendChatRequest();
  // TODO: make a selector for this, or show tool formation
  const thread = useAppSelector(selectThread);
  const isIntegration = thread.integration ?? false;

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
        if (!isIntegration && !areToolsConfirmed) {
          abort();
          if (recallCounter < 1) {
            recallCounter++;
            return;
          }
        }
        void sendMessages(currentMessages);
        recallCounter = 0;
      }
    }
  }, [
    errored,
    currentMessages,
    preventSend,
    sendMessages,
    abort,
    streaming,
    areToolsConfirmed,
    isWaiting,
    isIntegration,
  ]);
}
