import React, { useCallback, useRef, useEffect } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { Flex, Button, Text, Container, Card } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { PageWrapper } from "../PageWrapper";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import type { Config } from "../../features/Config/configSlice";
import { useEventsBusForIDE } from "../../hooks";
import {
  enableSend,
  getSelectedChatModel,
  newChatAction,
  selectIsStreaming,
  selectIsWaiting,
  setChatModel,
  useSendChatRequest,
  // selectThread,
  selectPreventSend,
  selectChatId,
  selectMessages,
} from "../../features/Chat/chatThread";
import { selectSelectedSnippet } from "../../features/Chat/selectedSnippet";
import { selectActiveFile } from "../../features/Chat/activeFile";
import { useTourRefs } from "../../features/Tour";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  // TODO: this
  // openChatInNewTab: () => void;
  style?: React.CSSProperties;

  unCalledTools: boolean;
  // enableSend: (value: boolean) => void;

  // TODO: update this
  caps: ChatFormProps["caps"];
  commands: ChatFormProps["commands"];

  requestCommandsCompletion: ChatFormProps["requestCommandsCompletion"];

  maybeSendToSidebar: ChatFormProps["onClose"];

  filesInPreview: ChatFormProps["filesInPreview"];

  prompts: ChatFormProps["prompts"];

  onSetSystemPrompt: ChatFormProps["onSetSystemPrompt"];
  selectedSystemPrompt: ChatFormProps["selectedSystemPrompt"];
  requestPreviewFiles: ChatFormProps["requestPreviewFiles"];
};

export const Chat: React.FC<ChatProps> = ({
  style,
  host,

  backFromChat,

  unCalledTools,

  caps,
  commands,

  requestCommandsCompletion,

  maybeSendToSidebar,

  filesInPreview,

  prompts,

  onSetSystemPrompt,
  selectedSystemPrompt,
  requestPreviewFiles,
}) => {
  const chatContentRef = useRef<HTMLDivElement>(null);
  const activeFile = useAppSelector(selectActiveFile);
  const selectedSnippet = useAppSelector(selectSelectedSnippet);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const canPaste = activeFile.can_paste;
  const chatId = useAppSelector(selectChatId);
  const { submit, abort, retry } = useSendChatRequest();
  const chatModel = useAppSelector(getSelectedChatModel);
  const dispatch = useAppDispatch();
  const messages = useAppSelector(selectMessages);
  const onSetChatModel = useCallback(
    (value: string) => {
      const model = caps.default_cap === value ? "" : value;
      dispatch(setChatModel(model));
    },
    [caps.default_cap, dispatch],
  );
  const preventSend = useAppSelector(selectPreventSend);
  const onEnableSend = () => dispatch(enableSend({ id: chatId }));
  const refs = useTourRefs();

  const {
    diffPasteBack,
    newFile,
    openSettings,
    openFile,
    // openChatInNewTab: _openChatInNewTab,
  } = useEventsBusForIDE();

  // const handleOpenChatInNewTab = useCallback(() => {
  //   openChatInNewTab(chatThread);
  //   // TODO: navigate to history
  // }, [chatThread, openChatInNewTab]);

  // TODO: add other posable errors
  // const onClearError = () => dispatch(clearChatError({ id: chatId }));
  // TODO: add other posable errors
  // const error = useAppSelector((state) => state.chat.error ?? caps.error);

  // TODO: handle stop
  const handleSummit = useCallback(
    (value: string) => {
      submit(value);
    },
    [submit],
  );

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
      // TODO: could be improved
      const action = newChatAction();
      dispatch(action);
    },
    [dispatch],
  );

  const focusTextarea = useCallback(() => {
    const textarea = document.querySelector<HTMLTextAreaElement>(
      '[data-testid="chat-form-textarea"]',
    );
    if (textarea) {
      textarea.focus();
    }
  }, []);

  useEffect(() => {
    if (!isWaiting && !isStreaming) {
      focusTextarea();
    }
  }, [isWaiting, isStreaming, focusTextarea]);

  return (
    <PageWrapper host={host} style={style}>
      {/* {host === "vscode" && !tabbed && ( */}
      <Flex gap="2" pb="3" wrap="wrap">
        <Button
          size="1"
          variant="surface"
          onClick={backFromChat}
          ref={(x) => refs.setBack(x)}
        >
          <ArrowLeftIcon width="16" height="16" />
          Back
        </Button>
        {/* {host === "vscode" && (
          <Button
            size="1"
            variant="surface"
            onClick={handleOpenChatInNewTab}
            ref={(x) => refs.setOpenInNewTab(x)}
          >
            Open In Tab
          </Button>
        )} */}
        <Button
          size="1"
          variant="surface"
          onClick={handleNewChat}
          ref={(x) => refs.setNewChatInside(x)}
        >
          New Chat
        </Button>
      </Flex>
      {/* )} */}
      <ChatContent
        key={`chat-content-${chatId}`}
        chatKey={chatId}
        // messages={chat.messages}
        // could be moved down
        onRetry={retry}
        isWaiting={isWaiting}
        isStreaming={isStreaming}
        onNewFileClick={newFile}
        onPasteClick={diffPasteBack}
        canPaste={canPaste}
        ref={chatContentRef}
        openSettings={openSettings}
        onOpenFile={openFile}
      />
      {!isStreaming && preventSend && unCalledTools && (
        <Container py="4" bottom="0" style={{ justifyContent: "flex-end" }}>
          <Card>
            <Flex direction="column" align="center" gap="2">
              Chat was interrupted with uncalled tools calls.
              <Button onClick={onEnableSend}>Resume</Button>
            </Flex>
          </Card>
        </Container>
      )}
      <ChatForm
        // todo: find a way to not have to stringify the whole caps object
        // the reason is that otherwise the tour bubbles will be in the wrong position due to layout shifts
        key={`chat-form-${chatId}-${JSON.stringify(caps)}`}
        chatId={chatId}
        isStreaming={isStreaming}
        showControls={messages.length === 0 && !isStreaming}
        onSubmit={handleSummit}
        model={chatModel}
        onSetChatModel={onSetChatModel}
        caps={caps}
        onStopStreaming={abort}
        commands={commands}
        requestCommandsCompletion={requestCommandsCompletion}
        onClose={maybeSendToSidebar}
        filesInPreview={filesInPreview}
        selectedSnippet={selectedSnippet}
        onTextAreaHeightChange={onTextAreaHeightChange}
        prompts={prompts}
        onSetSystemPrompt={onSetSystemPrompt}
        selectedSystemPrompt={selectedSystemPrompt}
        requestPreviewFiles={requestPreviewFiles}
      />
      <Flex justify="between" pl="1" pr="1" pt="1">
        {messages.length > 0 && (
          <Text size="1">model: {chatModel || caps.default_cap} </Text>
        )}
      </Flex>
    </PageWrapper>
  );
};
