import React, { useCallback, useState } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { Flex } from "@radix-ui/themes";
import { useAppSelector, useSendMessages } from "../../hooks";
import { type Config } from "../../features/Config/configSlice";

import { DropzoneProvider } from "../Dropzone";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { Checkpoints } from "../../features/Checkpoints";
// TODO: remove this?
// import { SuggestNewChat } from "../ChatForm/SuggestNewChat";
import { useMessageSubscription } from "./useMessageSubscription";
import { selectThreadId } from "../../features/ThreadMessages";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  style?: React.CSSProperties;

  maybeSendToSidebar: ChatFormProps["onClose"];
};

export const Chat: React.FC<ChatProps> = ({ style, maybeSendToSidebar }) => {
  // const dispatch = useAppDispatch();
  // const unCalledTools = useAppSelector(selectBranchHasUncalledTools);

  const [isViewingRawJSON, setIsViewingRawJSON] = useState(false);
  // const isStreaming = useAppSelector(selectIsStreaming);
  useMessageSubscription();
  const { sendMessage } = useSendMessages();
  // const totalMessages = useAppSelector(selectTotalMessagesInThread, {
  //   devModeChecks: { stabilityCheck: "never" },
  // });

  const chatId = useAppSelector(selectThreadId);
  // TODO: figure out features removed here
  // const { submit, abort, retryFromIndex } = useSendChatRequest();

  // const chatToolUse = useAppSelector(getSelectedToolUse);
  // const threadNewChatSuggested = useAppSelector(selectThreadNewChatSuggested);
  //   const messages = useAppSelector(selectMessages);
  // const capsForToolUse = useCapsForToolUse();

  const { shouldCheckpointsPopupBeShown } = useCheckpoints();

  // const [isDebugChatHistoryVisible, setIsDebugChatHistoryVisible] =
  //   useState(false);

  // const preventSend = useAppSelector(selectPreventSend);
  // const onEnableSend = () => dispatch(enableSend({ id: chatId }));

  const handleSummit = useCallback(
    (value: string) => {
      // submit({ question: value });
      void sendMessage(value);
      if (isViewingRawJSON) {
        setIsViewingRawJSON(false);
      }
    },
    [sendMessage, isViewingRawJSON],
  );

  // TODO: this
  // const handleThreadHistoryPage = useCallback(() => {
  //   dispatch(push({ name: "thread history page", chatId }));
  // }, [chatId, dispatch]);

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
        <ChatContent key={`chat-content-${chatId ?? "new"}`} />

        {shouldCheckpointsPopupBeShown && <Checkpoints />}

        {/* <SuggestNewChat
          shouldBeVisible={
            threadNewChatSuggested.wasSuggested &&
            !threadNewChatSuggested.wasRejectedByUser
          }
        /> */}

        <ChatForm
          key={chatId} // TODO: think of how can we not trigger re-render on chatId change (checkboxes)
          onSubmit={handleSummit}
          onClose={maybeSendToSidebar}
        />

        {/* <Flex justify="between" pl="1" pr="1" pt="1"> */}
        {/* Two flexboxes are left for the future UI element on the right side */}
        {/* {totalMessages > 0 && (
            <Flex align="center" justify="between" width="100%">
              <Flex align="center" gap="1">
                <Text size="1">model: {capsForToolUse.currentModel} </Text> •{" "}
                <Text
                  size="1"
                  onClick={() => setIsDebugChatHistoryVisible((prev) => !prev)}
                >
                  mode: {chatToolUse}{" "}
                </Text>
              </Flex>
              {totalMessages !== 0 &&
                !isStreaming &&
                isDebugChatHistoryVisible && (
                  <ThreadHistoryButton
                    title="View history of current thread"
                    size="1"
                    onClick={handleThreadHistoryPage}
                  />
                )}
            </Flex>
          )} */}
        {/* </Flex> */}
      </Flex>
    </DropzoneProvider>
  );
};
