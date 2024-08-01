import React, { useCallback, useRef } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent, ChatContentProps } from "../ChatContent";
import { Flex, Button, Text, Container, Card } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { PageWrapper } from "../PageWrapper";
import { type Config } from "../../events";
import { ChatState, ChatCapsState } from "../../hooks";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  openChatInNewTab: () => void;
  style?: React.CSSProperties;
  onStartNewChat: () => void;
  preventSend: boolean;
  unCalledTools: boolean;
  enableSend: (value: boolean) => void;

  chat: ChatState["chat"];
  error: ChatState["error"];
  // TODO: update this
  caps: ChatCapsState;
  // commands: ChatState["commands"];
  commands: ChatFormProps["commands"];

  retryQuestion: ChatContentProps["onRetry"];
  isWaiting: ChatContentProps["isWaiting"];
  isStreaming: ChatContentProps["isStreaming"];
  onNewFileClick: ChatContentProps["onNewFileClick"];
  onPasteClick: ChatContentProps["onPasteClick"];
  canPaste: ChatContentProps["canPaste"];
  openSettings: ChatContentProps["openSettings"];

  hasContextFile: ChatFormProps["hasContextFile"];
  requestCommandsCompletion: ChatFormProps["requestCommandsCompletion"];
  // setSelectedCommand: ChatFormProps["setSelectedCommand"];
  maybeSendToSidebar: ChatFormProps["onClose"];
  activeFile: ChatFormProps["attachFile"];
  filesInPreview: ChatFormProps["filesInPreview"];
  selectedSnippet: ChatFormProps["selectedSnippet"];
  removePreviewFileByName: ChatFormProps["removePreviewFileByName"];
  requestCaps: ChatFormProps["requestCaps"];
  prompts: ChatFormProps["prompts"];
  onSetSystemPrompt: ChatFormProps["onSetSystemPrompt"];
  selectedSystemPrompt: ChatFormProps["selectedSystemPrompt"];
  requestPreviewFiles: ChatFormProps["requestPreviewFiles"];
  canUseTools: ChatFormProps["canUseTools"];
  setUseTools: ChatFormProps["setUseTools"];
  useTools: ChatFormProps["useTools"];
  onSetChatModel: ChatFormProps["onSetChatModel"];
  onAskQuestion: ChatFormProps["onSubmit"];
  onClearError: ChatFormProps["clearError"];
  onStopStreaming: ChatFormProps["onStopStreaming"];
};

export const Chat: React.FC<ChatProps> = ({
  style,
  host,
  tabbed,
  backFromChat,
  openChatInNewTab,
  onStopStreaming,
  chat,
  error,
  onClearError,
  retryQuestion,
  isWaiting,
  isStreaming,
  onNewFileClick,
  onPasteClick,
  canPaste,
  preventSend,
  unCalledTools,
  enableSend,
  onAskQuestion,
  onSetChatModel,
  caps,
  commands,
  hasContextFile,
  requestCommandsCompletion,
  // setSelectedCommand,
  maybeSendToSidebar,
  activeFile,
  filesInPreview,
  selectedSnippet,
  removePreviewFileByName,
  requestCaps,
  prompts,
  onStartNewChat,
  onSetSystemPrompt,
  selectedSystemPrompt,
  requestPreviewFiles,
  canUseTools,
  setUseTools,
  useTools,
  openSettings,
}) => {
  const chatContentRef = useRef<HTMLDivElement>(null);

  const onTextAreaHeightChange = useCallback(() => {
    if (!chatContentRef.current) return;
    // TODO: handle preventing scroll if the user is not on the bottom of the chat
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    chatContentRef.current.scrollIntoView &&
      chatContentRef.current.scrollIntoView({
        behavior: "instant",
        block: "end",
      });
  }, [chatContentRef]);

  const handleNewChat = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      event.currentTarget.blur();
      onStopStreaming();
      onStartNewChat();
      // TODO: improve this
      const textarea = document.querySelector<HTMLTextAreaElement>(
        '[data-testid="chat-form-textarea"]',
      );
      if (textarea !== null) {
        textarea.focus();
      }
    },
    [onStartNewChat, onStopStreaming],
  );

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
          <Button size="1" variant="surface" onClick={handleNewChat}>
            New Chat
          </Button>
        </Flex>
      )}
      <ChatContent
        key={`chat-content-${chat.id}`}
        chatKey={chat.id}
        messages={chat.messages}
        onRetry={retryQuestion}
        isWaiting={isWaiting}
        isStreaming={isStreaming}
        onNewFileClick={onNewFileClick}
        onPasteClick={onPasteClick}
        canPaste={canPaste}
        ref={chatContentRef}
        openSettings={openSettings}
      />
      {!isStreaming && preventSend && unCalledTools && (
        <Container py="4" bottom="0" style={{ justifyContent: "flex-end" }}>
          <Card>
            <Flex direction="column" align="center" gap="2">
              Chat was interupted with uncalled tools calls.
              <Button onClick={() => enableSend(true)}>Resume</Button>
            </Flex>
          </Card>
        </Container>
      )}
      <ChatForm
        key={`chat-form-${chat.id}`}
        chatId={chat.id}
        isStreaming={isStreaming}
        showControls={chat.messages.length === 0 && !isStreaming}
        error={error}
        clearError={onClearError}
        onSubmit={onAskQuestion}
        model={chat.model}
        onSetChatModel={onSetChatModel}
        caps={caps}
        onStopStreaming={onStopStreaming}
        commands={commands}
        hasContextFile={hasContextFile}
        requestCommandsCompletion={requestCommandsCompletion}
        // setSelectedCommand={setSelectedCommand}
        onClose={maybeSendToSidebar}
        attachFile={activeFile}
        filesInPreview={filesInPreview}
        selectedSnippet={selectedSnippet}
        removePreviewFileByName={removePreviewFileByName}
        onTextAreaHeightChange={onTextAreaHeightChange}
        requestCaps={requestCaps}
        prompts={prompts}
        onSetSystemPrompt={onSetSystemPrompt}
        selectedSystemPrompt={selectedSystemPrompt}
        requestPreviewFiles={requestPreviewFiles}
        canUseTools={canUseTools}
        setUseTools={setUseTools}
        useTools={useTools}
      />
      <Flex justify="between" pl="1" pr="1" pt="1">
        {chat.messages.length > 0 && (
          <Text size="1">model: {chat.model || caps.default_cap} </Text>
        )}
      </Flex>
    </PageWrapper>
  );
};
