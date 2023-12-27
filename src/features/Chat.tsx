import React from "react";
import { ChatForm } from "../components/ChatForm";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import { ChatContent } from "../components/ChatContent";
import { Flex } from "@radix-ui/themes";

export const Chat: React.FC<{ style?: React.CSSProperties }> = (props) => {
  const {
    state,
    askQuestion,
    sendMessages,
    clearError,
    setChatModel,
    stopStreaming,
  } = useEventBusForChat();

  return (
    <Flex
      direction="column"
      justify="between"
      grow="1"
      p={{
        initial: "9",
      }}
      style={{
        ...props.style,
        height: "100dvh",
      }}
    >
      <ChatContent
        messages={state.chat.messages}
        onRetry={(messages) => sendMessages(messages)}
        isWaiting={state.waiting_for_response}
      />

      <ChatForm
        isStreaming={state.streaming}
        canChangeModel={state.chat.messages.length === 0 && !state.streaming}
        error={state.error}
        clearError={clearError}
        onSubmit={(value) => {
          askQuestion(value);
        }}
        model={state.chat.model}
        onSetChatModel={setChatModel}
        caps={state.caps}
        onStopStreaming={stopStreaming}
      />
    </Flex>
  );
};
