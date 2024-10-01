import { useCallback, useEffect, useMemo } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  getSelectedSystemPrompt,
  selectChatError,
  selectChatId,
  selectIsStreaming,
  selectMessages,
  selectPreventSend,
  selectSendImmediately,
  selectToolUse,
} from "../features/Chat/Thread/selectors";
import { useGetToolsQuery } from "./useGetToolsQuery";
import {
  ChatMessage,
  ChatMessages,
  DiffChunk,
  isAssistantMessage,
  isDiffMessage,
  isUserMessage,
} from "../services/refact/types";
import {
  backUpMessages,
  chatAskQuestionThunk,
  chatAskedQuestion,
  setToolUse,
} from "../features/Chat/Thread/actions";
import { takeFromLast } from "../utils/takeFromLast";
import { diffApi, DiffStateResponse } from "../services/refact/diffs";
import { isToolUse } from "../features/Chat";
import { useAbortControllers } from "./useAbortControllers";

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const hasError = useAppSelector(selectChatError);
  const abortControllers = useAbortControllers();

  const [getDiffState] = diffApi.useLazyDiffStateQuery();

  const toolsRequest = useGetToolsQuery();

  const chatId = useAppSelector(selectChatId);
  const streaming = useAppSelector(selectIsStreaming);
  const chatError = useAppSelector(selectChatError);

  const errored: boolean = !!hasError || !!chatError;
  const preventSend = useAppSelector(selectPreventSend);

  const currentMessages = useAppSelector(selectMessages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);
  const sendImmediately = useAppSelector(selectSendImmediately);
  const toolUse = useAppSelector(selectToolUse);

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
    (messages: ChatMessages) => {
      let tools = toolsRequest.data ?? null;
      if (isToolUse(toolUse)) {
        dispatch(setToolUse(toolUse));
      }
      if (toolUse === "quick") {
        tools = [];
      } else if (toolUse === "explore") {
        tools = tools?.filter((t) => !t.function.agentic) ?? [];
      }
      tools =
        tools?.map((t) => {
          const { agentic: _, ...remaining } = t.function;
          return { ...t, function: { ...remaining } };
        }) ?? [];
      dispatch(backUpMessages({ id: chatId, messages }));
      dispatch(chatAskedQuestion({ id: chatId }));

      const action = chatAskQuestionThunk({
        messages,
        tools,
        chatId,
      });

      const dispatchedAction = dispatch(action);
      abortControllers.addAbortController(chatId, dispatchedAction.abort);
    },
    [toolsRequest.data, toolUse, dispatch, chatId, abortControllers],
  );

  const submit = useCallback(
    async (question: string) => {
      const lastDiffs = takeFromLast(
        messagesWithSystemPrompt,
        isUserMessage,
      ).filter(isDiffMessage);

      if (lastDiffs.length === 0) {
        const message: ChatMessage = { role: "user", content: question };

        const messages = messagesWithSystemPrompt.concat(message);
        sendMessages(messages);
        return;
      }

      const chunks = lastDiffs.reduce<DiffChunk[]>((acc, cur) => {
        return [...acc, ...cur.content];
      }, []);

      const status = await getDiffState({ chunks }, true)
        .unwrap()
        .catch(() => [] as DiffStateResponse[]);

      const appliedChunks = status.filter((chunk) => chunk.state);

      const diffInfo = appliedChunks.map((diff) => {
        return `Preformed ${diff.chunk.file_action} on ${diff.chunk.file_name} at line ${diff.chunk.line1} to line ${diff.chunk.line2}.`;
      });

      const notAppliedMessage = "💿 user didn't accept the changes in the UI.";
      const appliedMessage =
        "💿 user accepted the following changes in the UI.\n" +
        diffInfo.join("\n");

      const diffMessage =
        appliedChunks.length === 0 ? notAppliedMessage : appliedMessage;

      const message: ChatMessage = {
        role: "user",
        content: diffMessage + "\n\n" + question,
      };
      const messages = messagesWithSystemPrompt.concat(message);
      sendMessages(messages);
    },
    [getDiffState, messagesWithSystemPrompt, sendMessages],
  );

  useEffect(() => {
    if (sendImmediately) {
      sendMessages(messagesWithSystemPrompt);
    }
  }, [sendImmediately, sendMessages, messagesWithSystemPrompt]);

  // TODO: Automatically calls tool calls. This means that this hook can only be used once :/
  useEffect(() => {
    if (!streaming && currentMessages.length > 0 && !errored && !preventSend) {
      const lastMessage = currentMessages.slice(-1)[0];
      if (
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls &&
        lastMessage.tool_calls.length > 0
      ) {
        sendMessages(currentMessages);
      }
    }
  }, [errored, currentMessages, preventSend, sendMessages, streaming]);

  const abort = useCallback(() => {
    abortControllers.abort(chatId);
  }, [abortControllers, chatId]);

  const retry = useCallback(
    (messages: ChatMessages) => {
      abort();
      sendMessages(messages);
    },
    [abort, sendMessages],
  );

  const retryFromIndex = useCallback(
    (index: number, question: string) => {
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
  };
};
