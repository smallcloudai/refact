import { useCallback, useEffect, useMemo } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  getSelectedSystemPrompt,
  selectAutomaticPatch,
  selectChatError,
  selectChatId,
  selectCheckpointsEnabled,
  selectHasUncalledTools,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectPreventSend,
  selectQueuedMessages,
  selectSendImmediately,
  selectThread,
  selectThreadMode,
  selectThreadToolUse,
  selectThreadConfirmationStatus,
  selectThreadImages,
  selectThreadPause,
} from "../features/Chat/Thread/selectors";
import { useCheckForConfirmationMutation } from "./useGetToolGroupsQuery";
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
  enqueueUserMessage,
  dequeueUserMessage,
  setThreadPauseReasons,
  clearThreadPauseReasons,
  setThreadConfirmationStatus,
} from "../features/Chat/Thread/actions";

import { useAbortControllers } from "./useAbortControllers";
import {
  chatModeToLspMode,
  doneStreaming,
  fixBrokenToolMessages,
  LspChatMode,
  setChatMode,
  setIsWaitingForResponse,
  setLastUserMessageId,
  setPreventSend,
  upsertToolCall,
} from "../features/Chat";

import { v4 as uuidv4 } from "uuid";
import { upsertToolCallIntoHistory } from "../features/History/historySlice";

type SendPolicy = "immediate" | "after_flow";

type SubmitHandlerParams =
  | {
      question: string;
      maybeMode?: LspChatMode;
      maybeMessages?: undefined;
      maybeDropLastMessage?: boolean;
      sendPolicy?: SendPolicy;
    }
  | {
      question?: undefined;
      maybeMode?: LspChatMode;
      maybeMessages?: undefined;
      maybeDropLastMessage?: boolean;
      sendPolicy?: SendPolicy;
    }
  | {
      question?: undefined;
      maybeMode?: LspChatMode;
      maybeMessages: ChatMessage[];
      maybeDropLastMessage?: boolean;
      sendPolicy?: SendPolicy;
    };

export const PATCH_LIKE_FUNCTIONS = [
  "patch",
  "text_edit",
  "create_textdoc",
  "update_textdoc",
  "replace_textdoc",
  "update_textdoc_regex",
  "update_textdoc_by_lines",
];

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const abortControllers = useAbortControllers();

  // const [triggerGetTools] = useGetToolsLazyQuery();
  const [triggerCheckForConfirmation] = useCheckForConfirmationMutation();

  const chatId = useAppSelector(selectChatId);

  const isWaiting = useAppSelector(selectIsWaiting);
  const isStreaming = useAppSelector(selectIsStreaming);
  const hasUnsentTools = useAppSelector(selectHasUncalledTools);

  const isBusy = isWaiting || isStreaming || hasUnsentTools;

  const currentMessages = useAppSelector(selectMessages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);
  const toolUse = useAppSelector(selectThreadToolUse);
  const attachedImages = useAppSelector(selectThreadImages);
  const threadMode = useAppSelector(selectThreadMode);
  const threadIntegration = useAppSelector(selectIntegration);
  const confirmationStatus = useAppSelector(selectThreadConfirmationStatus);
  const wasInteracted = confirmationStatus.wasInteracted;
  const areToolsConfirmed = confirmationStatus.confirmationStatus;

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
      dispatch(setIsWaitingForResponse({ id: chatId, value: true }));
      const lastMessage = messages.slice(-1)[0];

      if (
        !isWaiting &&
        !wasInteracted &&
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls &&
        lastMessage.tool_calls.length > 0
      ) {
        const toolCalls = lastMessage.tool_calls;
        const firstToolCall = toolCalls[0];
        // Safety check for incomplete tool calls (can happen after aborted streams)
        const firstToolName = firstToolCall?.function?.name;
        if (
          !(
            firstToolName &&
            PATCH_LIKE_FUNCTIONS.includes(firstToolName) &&
            isPatchAutomatic
          )
        ) {
          const confirmationResponse = await triggerCheckForConfirmation({
            tool_calls: toolCalls,
            messages: messages,
          }).unwrap();
          if (confirmationResponse.pause) {
            dispatch(setThreadPauseReasons({ id: chatId, pauseReasons: confirmationResponse.pause_reasons }));
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
        checkpointsEnabled,
        chatId,
        mode,
      });

      const dispatchedAction = dispatch(action);
      abortControllers.addAbortController(chatId, dispatchedAction.abort);
    },
    [
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
      sendPolicy = "after_flow",
    }: SubmitHandlerParams) => {
      let messages = messagesWithSystemPrompt;
      if (maybeDropLastMessage) {
        messages = messages.slice(0, -1);
      }

      if (question) {
        const message = maybeAddImagesToQuestion(question);

        // If busy, queue the message (priority = send at next available turn)
        if (isBusy) {
          dispatch(
            enqueueUserMessage({
              id: uuidv4(),
              message,
              createdAt: Date.now(),
              priority: sendPolicy === "immediate",
            }),
          );
          return;
        }

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
      isBusy,
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
    dispatch(setPreventSend({ id: chatId }));
    dispatch(fixBrokenToolMessages({ id: chatId }));
    dispatch(setIsWaitingForResponse({ id: chatId, value: false }));
    dispatch(doneStreaming({ id: chatId }));
  }, [abortControllers, chatId, dispatch]);

  const retry = useCallback(
    (messages: ChatMessages) => {
      abort();
      dispatch(clearThreadPauseReasons({ id: chatId }));
      dispatch(setThreadConfirmationStatus({ id: chatId, wasInteracted: false, confirmationStatus: areToolsConfirmed }));
      void sendMessages(messages);
    },
    [abort, sendMessages, dispatch, chatId, areToolsConfirmed],
  );

  const confirmToolUsage = useCallback(() => {
    dispatch(clearThreadPauseReasons({ id: chatId }));
    dispatch(setThreadConfirmationStatus({ id: chatId, wasInteracted: true, confirmationStatus: true }));
    // Continue the conversation - sendMessages will set waiting=true and proceed
    // since wasInteracted is now true, the confirmation check will be skipped
    void sendMessages(currentMessages);
  }, [dispatch, chatId, sendMessages, currentMessages]);

  const rejectToolUsage = useCallback(
    (toolCallIds: string[]) => {
      toolCallIds.forEach((toolCallId) => {
        dispatch(upsertToolCallIntoHistory({ toolCallId, chatId, accepted: false }));
        dispatch(upsertToolCall({ toolCallId, chatId, accepted: false }));
      });

      dispatch(clearThreadPauseReasons({ id: chatId }));
      dispatch(setThreadConfirmationStatus({ id: chatId, wasInteracted: false, confirmationStatus: true }));
      dispatch(setIsWaitingForResponse({ id: chatId, value: false }));
      dispatch(doneStreaming({ id: chatId }));
      dispatch(setPreventSend({ id: chatId }));
    },
    [chatId, dispatch],
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

export function useAutoSend() {
  const dispatch = useAppDispatch();
  const streaming = useAppSelector(selectIsStreaming);
  const currentMessages = useAppSelector(selectMessages);
  const errored = useAppSelector(selectChatError);
  const preventSend = useAppSelector(selectPreventSend);
  const isWaiting = useAppSelector(selectIsWaiting);
  const sendImmediately = useAppSelector(selectSendImmediately);
  const confirmationStatus = useAppSelector(selectThreadConfirmationStatus);
  const wasInteracted = confirmationStatus.wasInteracted;
  const areToolsConfirmed = confirmationStatus.confirmationStatus;
  const isPaused = useAppSelector(selectThreadPause);
  const hasUnsentTools = useAppSelector(selectHasUncalledTools);
  const queuedMessages = useAppSelector(selectQueuedMessages);
  const { sendMessages, messagesWithSystemPrompt } = useSendChatRequest();
  const thread = useAppSelector(selectThread);
  const isIntegration = thread?.integration ?? false;

  useEffect(() => {
    if (sendImmediately) {
      dispatch(setSendImmediately(false));
      void sendMessages(messagesWithSystemPrompt);
    }
  }, [dispatch, messagesWithSystemPrompt, sendImmediately, sendMessages]);

  const stop = useMemo(() => {
    if (errored) return true;
    if (preventSend) return true;
    if (isWaiting) return true;
    if (streaming) return true;
    return !hasUnsentTools;
  }, [errored, hasUnsentTools, isWaiting, preventSend, streaming]);

  const stopForToolConfirmation = useMemo(() => {
    if (isIntegration) return false;
    if (isPaused) return true;
    return !wasInteracted && !areToolsConfirmed;
  }, [isIntegration, isPaused, wasInteracted, areToolsConfirmed]);

  // Base conditions for flushing queue (streaming must be done)
  const canFlushBase = useMemo(() => {
    if (errored) return false;
    if (preventSend) return false;
    if (streaming) return false;
    if (isWaiting) return false;
    return true;
  }, [errored, preventSend, streaming, isWaiting]);

  // Full idle: also wait for tools to complete (for regular queued messages)
  const isFullyIdle = useMemo(() => {
    if (!canFlushBase) return false;
    if (hasUnsentTools) return false;
    if (stopForToolConfirmation) return false;
    return true;
  }, [canFlushBase, hasUnsentTools, stopForToolConfirmation]);

  // Process queued messages
  // Priority messages: flush as soon as streaming ends (next turn)
  // Regular messages: wait for full idle (tools complete)
  useEffect(() => {
    if (queuedMessages.length === 0) return;

    const nextQueued = queuedMessages[0];
    const isPriority = nextQueued.priority;

    // Priority: flush when base conditions met (right after streaming)
    // Regular: flush only when fully idle (after tools complete)
    const canFlush = isPriority ? canFlushBase : isFullyIdle;

    if (!canFlush) return;

    // Remove from queue first to prevent double-send
    dispatch(dequeueUserMessage({ queuedId: nextQueued.id }));

    // Send the queued message
    void sendMessages([...currentMessages, nextQueued.message], thread?.mode);
  }, [
    canFlushBase,
    isFullyIdle,
    queuedMessages,
    dispatch,
    sendMessages,
    currentMessages,
    thread?.mode,
  ]);

  // Check if there are priority messages waiting
  const hasPriorityMessages = useMemo(
    () => queuedMessages.some((m) => m.priority),
    [queuedMessages],
  );

  // NOTE: Tool auto-continue is handled by middleware (doneStreaming listener)
  // Having it here as well caused a race condition where both would fire,
  // resulting in two overlapping streaming requests that mixed up messages.
  // See middleware.ts doneStreaming listener for the single source of truth.

  // Export these for components that need to know idle state
  return { stop, stopForToolConfirmation, hasPriorityMessages };
}
