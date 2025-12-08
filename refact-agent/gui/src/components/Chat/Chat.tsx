import React, { useCallback, useState } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { Flex, Button, Text, Card } from "@radix-ui/themes";
import {
  useAppSelector,
  useAppDispatch,
  useSendChatRequest,
  useAutoSend,
} from "../../hooks";
import { type Config } from "../../features/Config/configSlice";
import {
  enableSend,
  selectIsStreaming,
  selectPreventSend,
  selectChatId,
  selectMessages,
  getSelectedToolUse,
  selectThreadNewChatSuggested,
} from "../../features/Chat/Thread";
import { ThreadHistoryButton } from "../Buttons";
import { push } from "../../features/Pages/pagesSlice";
import { DropzoneProvider } from "../Dropzone";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { Checkpoints } from "../../features/Checkpoints";
import { SuggestNewChat } from "../ChatForm/SuggestNewChat";
import { EnhancedModelSelector } from "./EnhancedModelSelector";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  style?: React.CSSProperties;
  unCalledTools: boolean;
  maybeSendToSidebar: ChatFormProps["onClose"];
};

export const Chat: React.FC<ChatProps> = ({
  style,
  unCalledTools,
  maybeSendToSidebar,
}) => {
  const dispatch = useAppDispatch();

  const [isViewingRawJSON, setIsViewingRawJSON] = useState(false);
  const isStreaming = useAppSelector(selectIsStreaming);

  const chatId = useAppSelector(selectChatId);
  const { submit, abort, retryFromIndex } = useSendChatRequest();

  const chatToolUse = useAppSelector(getSelectedToolUse);
  const threadNewChatSuggested = useAppSelector(selectThreadNewChatSuggested);
  const messages = useAppSelector(selectMessages);

  const { shouldCheckpointsPopupBeShown } = useCheckpoints();

  const [isDebugChatHistoryVisible, setIsDebugChatHistoryVisible] =
    useState(false);

  const preventSend = useAppSelector(selectPreventSend);
  const onEnableSend = () => dispatch(enableSend({ id: chatId }));

  const handleSubmit = useCallback(
    (value: string) => {
      submit({ question: value });
      if (isViewingRawJSON) {
        setIsViewingRawJSON(false);
      }
    },
    [submit, isViewingRawJSON],
  );

  const handleThreadHistoryPage = useCallback(() => {
    dispatch(push({ name: "thread history page", chatId }));
  }, [chatId, dispatch]);

  useAutoSend();

  return (
    <DropzoneProvider asChild>
      <Flex
        style={style}
        direction="column"
        flexGrow="1"
        width="100%"
        overflowY="auto"
        justify="between"
        px="1"
      >
        <ChatContent
          key={`chat-content-${chatId}`}
          onRetry={retryFromIndex}
          onStopStreaming={abort}
        />

        {shouldCheckpointsPopupBeShown && <Checkpoints />}

        <SuggestNewChat
          shouldBeVisible={
            threadNewChatSuggested.wasSuggested &&
            !threadNewChatSuggested.wasRejectedByUser
          }
        />
        {!isStreaming && preventSend && unCalledTools && (
          <Flex py="4">
            <Card style={{ width: "100%" }}>
              <Flex direction="column" align="center" gap="2" width="100%">
                Chat was interrupted with uncalled tools calls.
                <Button onClick={onEnableSend}>Resume</Button>
              </Flex>
            </Card>
          </Flex>
        )}

        <ChatForm
          key={chatId} // TODO: think of how can we not trigger re-render on chatId change (checkboxes)
          onSubmit={handleSubmit}
          onClose={maybeSendToSidebar}
          unCalledTools={unCalledTools}
        />

        <Flex justify="between" pl="1" pr="1" pt="1">
          {/* Two flexboxes are left for the future UI element on the right side */}
          {messages.length > 0 && (
            <Flex align="center" justify="between" width="100%">
              <Flex align="center" gap="2">
                <EnhancedModelSelector disabled={isStreaming} />
                <Text size="1" color="gray">
                  •
                </Text>
                <Text
                  size="1"
                  color="gray"
                  onClick={() => setIsDebugChatHistoryVisible((prev) => !prev)}
                  style={{ cursor: "pointer" }}
                >
                  mode: {chatToolUse}
                </Text>
              </Flex>
              {messages.length !== 0 &&
                !isStreaming &&
                isDebugChatHistoryVisible && (
                  <ThreadHistoryButton
                    title="View history of current thread"
                    size="1"
                    onClick={handleThreadHistoryPage}
                  />
                )}
            </Flex>
          )}
        </Flex>
      </Flex>
    </DropzoneProvider>
  );
};
