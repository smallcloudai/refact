import { memo } from "react";
import { Flex, Box } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import {
  getHistory,
  type HistoryState,
} from "../../features/History/historySlice";
import type { ChatThread } from "../../features/Chat/Thread/types";

export type ChatHistoryProps = {
  history: HistoryState;
  onHistoryItemClick: (id: ChatThread) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
  currentChatId?: string;
};

export const ChatHistory = memo(
  ({
    history,
    onHistoryItemClick,
    onDeleteHistoryItem,
    onOpenChatInTab,
    currentChatId,
  }: ChatHistoryProps) => {
    const sortedHistory = getHistory({ history });
    return (
      <Box
        style={{
          overflow: "hidden",
        }}
        pb="2"
        flexGrow="1"
      >
        <ScrollArea scrollbars="vertical">
          <Flex
            justify="center"
            align="center"
            pl="2"
            pr="2"
            direction="column"
          >
            {sortedHistory.map((item) => (
              <HistoryItem
                onClick={() => onHistoryItemClick(item)}
                onOpenInTab={onOpenChatInTab}
                onDelete={onDeleteHistoryItem}
                key={item.id}
                historyItem={item}
                disabled={item.id === currentChatId}
              />
            ))}
          </Flex>
        </ScrollArea>
      </Box>
    );
  },
);

ChatHistory.displayName = "ChatHistory";
