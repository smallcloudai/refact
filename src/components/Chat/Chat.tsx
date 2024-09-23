import React, { useCallback, useRef, useEffect } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { Flex, Button, Text, Container, Card } from "@radix-ui/themes";
import {
  useAppSelector,
  useAppDispatch,
  useSendChatRequest,
} from "../../hooks";
import type { Config } from "../../features/Config/configSlice";
import { useEventsBusForIDE } from "../../hooks";
import {
  enableSend,
  getSelectedChatModel,
  selectIsStreaming,
  selectIsWaiting,
  setChatModel,
  // selectThread,
  selectPreventSend,
  selectChatId,
  selectMessages,
} from "../../features/Chat/Thread";
import { selectActiveFile } from "../../features/Chat/activeFile";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  style?: React.CSSProperties;
  unCalledTools: boolean;
  // TODO: update this
  caps: ChatFormProps["caps"];
  maybeSendToSidebar: ChatFormProps["onClose"];
  prompts: ChatFormProps["prompts"];
  onSetSystemPrompt: ChatFormProps["onSetSystemPrompt"];
  selectedSystemPrompt: ChatFormProps["selectedSystemPrompt"];
};

export const Chat: React.FC<ChatProps> = ({
  style,
  unCalledTools,
  caps,
  maybeSendToSidebar,
  prompts,
  onSetSystemPrompt,
  selectedSystemPrompt,
}) => {
  const chatContentRef = useRef<HTMLDivElement>(null);
  const activeFile = useAppSelector(selectActiveFile);
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

  const {
    diffPasteBack,
    newFile,
    openSettings,
    // openChatInNewTab: _openChatInNewTab,
  } = useEventsBusForIDE();

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
    <Flex style={style} direction="column" flexGrow="1">
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
        // key={`chat-form-${chatId}-${JSON.stringify(caps)}`}
        chatId={chatId}
        isStreaming={isStreaming}
        showControls={messages.length === 0 && !isStreaming}
        onSubmit={handleSummit}
        model={chatModel}
        onSetChatModel={onSetChatModel}
        caps={caps}
        onStopStreaming={abort}
        onClose={maybeSendToSidebar}
        onTextAreaHeightChange={onTextAreaHeightChange}
        prompts={prompts}
        onSetSystemPrompt={onSetSystemPrompt}
        selectedSystemPrompt={selectedSystemPrompt}
      />
      <Flex justify="between" pl="1" pr="1" pt="1">
        {messages.length > 0 && (
          <Text size="1">model: {chatModel || caps.default_cap} </Text>
        )}
      </Flex>
    </Flex>
  );
};
