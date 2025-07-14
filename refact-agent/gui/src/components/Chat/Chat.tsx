import React, { useCallback, useMemo, useState } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { Flex } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector, useSendMessages } from "../../hooks";
import { selectConfig, type Config } from "../../features/Config/configSlice";

import { DropzoneProvider } from "../Dropzone";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { Checkpoints } from "../../features/Checkpoints";

import { useMessageSubscription } from "./useMessageSubscription";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectThreadId,
  selectTotalMessagesInThread,
} from "../../features/ThreadMessages";
import { ThreadHistoryButton } from "../Buttons";
import { push } from "../../features/Pages/pagesSlice";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  style?: React.CSSProperties;

  maybeSendToSidebar: ChatFormProps["onClose"];
};

export const Chat: React.FC<ChatProps> = ({ style, maybeSendToSidebar }) => {
  const dispatch = useAppDispatch();
  // const unCalledTools = useAppSelector(selectBranchHasUncalledTools);

  const [isViewingRawJSON, setIsViewingRawJSON] = useState(false);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  useMessageSubscription();
  const { sendMessage } = useSendMessages();
  const totalMessages = useAppSelector(selectTotalMessagesInThread, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const config = useAppSelector(selectConfig);

  const canShowDebugButton = useMemo(() => {
    if (config.host === "web") return true;
    if (config.features?.connections) return true;
    return !isWaiting && !isStreaming && totalMessages > 0;
  }, [
    config.features?.connections,
    config.host,
    isStreaming,
    isWaiting,
    totalMessages,
  ]);

  const chatId = useAppSelector(selectThreadId);

  const { shouldCheckpointsPopupBeShown } = useCheckpoints();

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
  const handleThreadHistoryPage = useCallback(() => {
    dispatch(push({ name: "thread history page", chatId: chatId ?? "" }));
  }, [chatId, dispatch]);

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

        <ChatForm
          key={chatId} // TODO: think of how can we not trigger re-render on chatId change (checkboxes)
          onSubmit={handleSummit}
          onClose={maybeSendToSidebar}
        />

        <Flex justify="between" pl="1" pr="1" pt="1">
          {/* Two flexboxes are left for the future UI element on the right side */}
          {canShowDebugButton && (
            <Flex align="center" justify="end" width="100%">
              <ThreadHistoryButton
                title="View history of current thread"
                size="1"
                onClick={handleThreadHistoryPage}
              />
            </Flex>
          )}
        </Flex>
      </Flex>
    </DropzoneProvider>
  );
};
