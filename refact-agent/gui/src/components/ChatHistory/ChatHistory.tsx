import React, { memo, useCallback, useEffect, useMemo } from "react";
import { Flex, Box } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
// import {
//   getHistory,
//   type HistoryState,
// } from "../../features/History/historySlice";
// import type { ChatThread } from "../../features/Chat/Thread/types";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  chatDbSelectors,
  chatDbActions,
} from "../../features/ChatDB/chatDbSlice";
import { subscribeToThreadsThunk } from "../../services/refact/chatdb";
import { push } from "../../features/Pages/pagesSlice";
import { CThread } from "../../services/refact/types";

// export type ChatHistoryProps = {
//   history: HistoryState;
//   onHistoryItemClick: (id: ChatThread) => void;
//   onDeleteHistoryItem: (id: string) => void;
//   onOpenChatInTab?: (id: string) => void;
//   currentChatId?: string;
// };

function useGetHistory() {
  // todo: search
  const dispatch = useAppDispatch();
  const history = useAppSelector(chatDbSelectors.getChats, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const isLoading = useAppSelector(chatDbSelectors.getLoading);

  useEffect(() => {
    const thunk = dispatch(subscribeToThreadsThunk());
    return () => {
      thunk.abort("unmounted");
    };
  }, [dispatch]);

  const onDeleteHistoryItem = useCallback(
    (id: string) => {
      dispatch(chatDbActions.deleteCThread(id));
    },
    [dispatch],
  );

  const onHistoryItemClick = useCallback(
    (thread: CThread) => {
      dispatch(push({ name: "chat", threadId: thread.cthread_id }));
    },
    [dispatch],
  );

  return {
    history,
    isLoading,
    onHistoryItemClick,
    onDeleteHistoryItem,
  };
}

export const ChatHistory: React.FC = () => {
  const { history, onHistoryItemClick, onDeleteHistoryItem } = useGetHistory();

  return (
    <Box
      style={{
        overflow: "hidden",
      }}
      pb="2"
      flexGrow="1"
    >
      <ScrollArea scrollbars="vertical">
        <Flex justify="center" align="center" pl="2" pr="2" direction="column">
          {history.map((item) => (
            <HistoryItem
              // onClick={() => onHistoryItemClick(item)}
              onClick={onHistoryItemClick}
              // onOpenInTab={onOpenChatInTab}
              onDelete={onDeleteHistoryItem}
              key={item.cthread_id}
              historyItem={item}
              // disabled={item.cthread_id === currentChatId}
            />
          ))}
        </Flex>
      </ScrollArea>
    </Box>
  );
};

ChatHistory.displayName = "ChatHistory";
