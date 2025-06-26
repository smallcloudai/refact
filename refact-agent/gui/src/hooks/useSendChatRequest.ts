import { useCallback } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  // selectAutomaticPatch,
  // selectChatError,
  selectChatId,
  selectCheckpointsEnabled,
  // selectHasUncalledTools,
  selectIntegration,
  // selectIsStreaming,
  // selectIsWaiting,
  selectMessages,
  // selectPreventSend,
  // selectSendImmediately,
  // selectThread,
  selectThreadMode,
  selectThreadToolUse,
} from "../features/Chat/Thread/selectors";
// import {
//   // selectIsStreaming,
//   selectIsWaiting,
// } from "../features/ThreadMessages";
// import { useCheckForConfirmationMutation } from "./useGetToolGroupsQuery";
import {
  ChatMessage,
  ChatMessages,
  //  isAssistantMessage,
  isUserMessage,
  UserMessage,
  UserMessageContentWithImage,
} from "../services/refact/types";
import {
  backUpMessages,
  chatAskQuestionThunk,
  chatAskedQuestion,
  // setSendImmediately,
} from "../features/Chat/Thread/actions";

import { selectAllImages } from "../features/AttachedImages";
import { useAbortControllers } from "./useAbortControllers";
// import {
//   clearPauseReasonsAndHandleToolsStatus,
//   getToolsConfirmationStatus,
//   getToolsInteractionStatus,
//   setPauseReasons,
// } from "../features/ToolConfirmation/confirmationSlice";
import {
  chatModeToLspMode,
  doneStreaming,
  fixBrokenToolMessages,
  LspChatMode,
  setChatMode,
  setIsWaitingForResponse,
  setLastUserMessageId,
  setPreventSend,
} from "../features/Chat";

import { v4 as uuidv4 } from "uuid";

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

  const chatId = useAppSelector(selectChatId);

  const currentMessages = useAppSelector(selectMessages);
  const toolUse = useAppSelector(selectThreadToolUse);
  const attachedImages = useAppSelector(selectAllImages);
  const threadMode = useAppSelector(selectThreadMode);
  const threadIntegration = useAppSelector(selectIntegration);

  const checkpointsEnabled = useAppSelector(selectCheckpointsEnabled);

  const sendMessages = useCallback(
    (messages: ChatMessages, maybeMode?: LspChatMode) => {
      dispatch(setIsWaitingForResponse(true));
      const lastMessage = messages.slice(-1)[0];

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
      dispatch,
      chatId,
      threadMode,
      checkpointsEnabled,
      abortControllers,
    ],
  );

  const maybeAddImagesToQuestion = useCallback(
    (question: string): UserMessage => {
      if (attachedImages.length === 0)
        return {
          ftm_role: "user" as const,
          ftm_content: question,
          checkpoints: [],
        };

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
        return { ftm_role: "user", ftm_content: question, checkpoints: [] };

      return {
        ftm_role: "user",
        ftm_content: [...images, { type: "text", text: question }],
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
      let messages = currentMessages;
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

      sendMessages(messages, mode);
    },
    [
      dispatch,
      maybeAddImagesToQuestion,
      currentMessages,
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
    dispatch(setIsWaitingForResponse(false));
    dispatch(doneStreaming({ id: chatId }));
  }, [abortControllers, chatId, dispatch]);

  const retry = useCallback(
    (messages: ChatMessages) => {
      abort();
      sendMessages(messages);
    },
    [abort, sendMessages],
  );

  const retryFromIndex = useCallback(
    (index: number, question: UserMessage["ftm_content"]) => {
      const messagesToKeep = currentMessages.slice(0, index);
      const messagesToSend = messagesToKeep.concat([
        { ftm_role: "user", ftm_content: question, checkpoints: [] },
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

    maybeAddImagesToQuestion,

    sendMessages,
  };
};
