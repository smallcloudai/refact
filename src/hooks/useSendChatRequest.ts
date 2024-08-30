import { useCallback, useEffect, useMemo, useRef } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  getSelectedSystemPrompt,
  selectChatError,
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectPreventSend,
  selectSendImmediately,
  selectToolUse,
} from "../features/Chat/Thread/selectors";
import { useGetToolsQuery } from "./useGetToolsQuery";
import {
  ChatMessage,
  ChatMessages,
  isAssistantMessage,
} from "../services/refact/types";
import {
  backUpMessages,
  chatAskQuestionThunk,
  chatAskedQuestion,
} from "../features/Chat/Thread/actions";

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const abortRef = useRef<null | ((reason?: string | undefined) => void)>(null);
  const hasError = useAppSelector(selectChatError);

  const toolsRequest = useGetToolsQuery();

  const chatId = useAppSelector(selectChatId);
  const streaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
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
      abortRef.current = dispatchedAction.abort;
    },
    [chatId, dispatch, toolsRequest.data, toolUse],
  );

  const submit = useCallback(
    (question: string) => {
      // const tools = toolsRequest.data ?? null;
      const message: ChatMessage = { role: "user", content: question };
      // This may cause duplicated messages
      const messages = messagesWithSystemPrompt.concat(message);
      sendMessages(messages);
    },
    [messagesWithSystemPrompt, sendMessages],
  );

  useEffect(() => {
    if (sendImmediately) {
      sendMessages(messagesWithSystemPrompt);
    }
  }, [sendImmediately, sendMessages, messagesWithSystemPrompt]);

  // Automatically calls tool calls.
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

  const abort = () => {
    if (abortRef.current && (streaming || isWaiting)) {
      abortRef.current();
    }
  };

  const retry = (messages: ChatMessages) => {
    abort();
    sendMessages(messages);
  };

  return {
    submit,
    abort,
    retry,
  };
};
