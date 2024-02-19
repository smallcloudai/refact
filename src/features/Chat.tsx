import React, { useRef } from "react";
import { ChatForm } from "../components/ChatForm";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import { ChatContent } from "../components/ChatContent";
import { Flex, Responsive, Button } from "@radix-ui/themes";
import { isChatContextFileMessage } from "../services/refact";
import { useConfig } from "../contexts/config-context";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useEffectOnce } from "../hooks";

export const Chat: React.FC<{ style?: React.CSSProperties }> = (props) => {
  useEffectOnce(() => {
    sendReadyMessage();
  });

  const { host, tabbed } = useConfig();

  const chatContentRef = useRef<HTMLDivElement>(null);

  const {
    state,
    askQuestion,
    clearError,
    setChatModel,
    stopStreaming,
    handleContextFile,
    hasContextFile,
    backFromChat,
    openChatInNewTab,
    sendToSideBar,
    sendReadyMessage,
    handleNewFileClick,
    handlePasteDiffClick,
    requestCommandsCompletion,
    setSelectedCommand,
    removePreviewFileByName,
    retryQuestion,
  } = useEventBusForChat();

  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

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
      {host === "vscode" && !tabbed && (
        <Flex gap="2" pb="3">
          <Button variant="surface" onClick={backFromChat}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
          <Button variant="surface" onClick={openChatInNewTab}>
            Open In Tab
          </Button>
        </Flex>
      )}
      <ChatContent
        messages={state.chat.messages}
        onRetry={retryQuestion}
        isWaiting={state.waiting_for_response}
        onNewFileClick={handleNewFileClick}
        onPasteClick={handlePasteDiffClick}
        canPaste={state.active_file.can_paste}
        ref={chatContentRef}
      />

      <ChatForm
        isStreaming={state.streaming}
        canChangeModel={
          state.chat.messages.filter(
            (message) => !isChatContextFileMessage(message),
          ).length === 0 &&
          !state.streaming &&
          state.files_in_preview.length === 0
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
        requestCommandsCompletion={requestCommandsCompletion}
        setSelectedCommand={setSelectedCommand}
        onClose={maybeSendToSideBar}
        attachFile={state.active_file}
        filesInPreview={state.files_in_preview}
        selectedSnippet={state.selected_snippet}
        removePreviewFileByName={removePreviewFileByName}
        onTextAreaHeightChange={() => {
          if (!chatContentRef.current) return;
          // TODO: handle preventing scroll if the user is not on the bottom of the chat
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          chatContentRef.current.scrollIntoView &&
            chatContentRef.current.scrollIntoView({
              behavior: "instant",
              block: "end",
            });
        }}
      />
    </Flex>
  );
};
