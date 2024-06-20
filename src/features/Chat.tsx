import React, { useMemo, useRef } from "react";
import { ChatForm } from "../components/ChatForm";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import { ChatContent } from "../components/ChatContent";
import { Flex, Button, Text } from "@radix-ui/themes";
import { useConfig } from "../contexts/config-context";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { PageWrapper } from "../components/PageWrapper";

export const Chat: React.FC<{ style?: React.CSSProperties }> = ({ style }) => {
  const { host, tabbed } = useConfig();

  const chatContentRef = useRef<HTMLDivElement>(null);

  const {
    state,
    askQuestion,
    clearError,
    setChatModel,
    stopStreaming,
    backFromChat,
    openChatInNewTab,
    sendToSideBar,
    handleNewFileClick,
    handlePasteDiffClick,
    hasContextFile,
    requestCommandsCompletion,
    requestPreviewFiles,
    setSelectedCommand,
    removePreviewFileByName,
    retryQuestion,
    maybeRequestCaps,
    startNewChat,
    setSelectedSystemPrompt,
    setUseTools,
  } = useEventBusForChat();

  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

  const canUseTools = useMemo(() => {
    if (state.tools === null || state.tools.length === 0) return false;
    const model = state.chat.model || state.caps.default_cap;
    if (state.caps.available_caps[model].supports_tools) return true;
    return false;
  }, [
    state.tools,
    state.chat.model,
    state.caps.default_cap,
    state.caps.available_caps,
  ]);

  return (
    <PageWrapper host={host} style={style}>
      {host === "vscode" && !tabbed && (
        <Flex gap="2" pb="3" wrap="wrap">
          <Button size="1" variant="surface" onClick={backFromChat}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
          <Button size="1" variant="surface" onClick={openChatInNewTab}>
            Open In Tab
          </Button>
          <Button
            size="1"
            variant="surface"
            onClick={(event) => {
              event.currentTarget.blur();
              stopStreaming();
              startNewChat();
              // TODO: improve this
              const textarea = document.querySelector<HTMLTextAreaElement>(
                '[data-testid="chat-form-textarea"]',
              );
              if (textarea !== null) {
                textarea.focus();
              }
            }}
          >
            New Chat
          </Button>
        </Flex>
      )}
      <ChatContent
        messages={state.chat.messages}
        onRetry={retryQuestion}
        isWaiting={state.waiting_for_response}
        isStreaming={state.streaming}
        onNewFileClick={handleNewFileClick}
        onPasteClick={handlePasteDiffClick}
        canPaste={state.active_file.can_paste}
        ref={chatContentRef}
      />

      <ChatForm
        chatId={state.chat.id}
        isStreaming={state.streaming}
        showControls={state.chat.messages.length === 0 && !state.streaming}
        error={state.error}
        clearError={clearError}
        onSubmit={(value) => {
          askQuestion(value);
        }}
        model={state.chat.model}
        onSetChatModel={setChatModel}
        caps={state.caps}
        onStopStreaming={stopStreaming}
        commands={state.commands}
        hasContextFile={hasContextFile}
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
        requestCaps={maybeRequestCaps}
        prompts={state.system_prompts.prompts}
        onSetSystemPrompt={setSelectedSystemPrompt}
        selectedSystemPrompt={state.selected_system_prompt}
        requestPreviewFiles={requestPreviewFiles}
        canUseTools={canUseTools}
        setUseTools={setUseTools}
        useTools={state.use_tools}
      />

      <Flex justify="between" pl="1" pr="1" pt="1">
        {state.chat.messages.length > 0 && (
          <Text size="1">
            model: {state.chat.model || state.caps.default_cap}{" "}
          </Text>
        )}
      </Flex>
    </PageWrapper>
  );
};
