import React from "react";
import { ChatForm } from "../components/ChatForm";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import { ChatContent } from "../components/ChatContent";
import { Flex } from "@radix-ui/themes";

export const Chat: React.FC = () => {
  const { state, askQuestion, sendMessages, clearError, setChatModel } =
    useEventBusForChat();

  return (
    <Flex
      direction="column"
      justify="between"
      grow="1"
      style={{
        height: "calc(100dvh - 180px)", // TODO: fix this
        // minHeight: "100%",
      }}
    >
      <ChatContent
        messages={state.chat.messages}
        onRetry={(messages) => sendMessages(messages)}
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
      />
    </Flex>
  );
};
