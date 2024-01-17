import React from "react";
import { ChatForm } from "../components/ChatForm";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import { ChatContent } from "../components/ChatContent";
import { Flex, Responsive } from "@radix-ui/themes";
import { isChatContextFileMessage } from "../services/refact";
import { useConfig } from "../contexts/config-context";

export const Chat: React.FC<{ style?: React.CSSProperties }> = (props) => {
  const { host } = useConfig();

  const {
    state,
    askQuestion,
    sendMessages,
    clearError,
    setChatModel,
    stopStreaming,
    handleContextFile,
    hasContextFile,
  } = useEventBusForChat();

  const LeftRightPadding: Responsive<
    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  > =
    host === "web"
      ? { initial: "8", xl: "9" }
      : {
          initial: "2",
          xs: "2",
          sm: "4",
          md: "8",
          lg: "8",
          xl: "9",
        };

  const TopBottomPadding: Responsive<
    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  > = {
    initial: "5",
    // xs: "1",
    // sm: "2",
    // md: "3",
    // lg: "4",
    // xl: "5",
  };

  return (
    <Flex
      direction="column"
      justify="between"
      grow="1"
      pr={LeftRightPadding}
      pl={LeftRightPadding}
      pt={TopBottomPadding}
      pb={TopBottomPadding}
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
        canChangeModel={
          state.chat.messages.filter(
            (message) => !isChatContextFileMessage(message),
          ).length === 0 && !state.streaming
        }
        error={state.error}
        clearError={clearError}
        onSubmit={(value) => {
          askQuestion(value);
        }}
        model={state.chat.model}
        onSetChatModel={setChatModel}
        caps={state.caps}
        onStopStreaming={stopStreaming}
        handleContextFile={handleContextFile}
        hasContextFile={hasContextFile}
        commands={state.rag_commands}
      />
    </Flex>
  );
};
