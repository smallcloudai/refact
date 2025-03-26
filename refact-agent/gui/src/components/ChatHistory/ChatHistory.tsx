import { memo } from "react";
import { Flex, Box, Text } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import {
  ChatHistoryItem,
  getHistory,
  type HistoryState,
} from "../../features/History/historySlice";

export type ChatHistoryProps = {
  history: HistoryState;
  onHistoryItemClick: (id: ChatHistoryItem) => void;
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
            align={sortedHistory.length > 0 ? "center" : "start"}
            pl="2"
            pr="2"
            direction="column"
          >
            {sortedHistory.length !== 0 ? (
              sortedHistory.map((item) => (
                <HistoryItem
                  onClick={() => onHistoryItemClick(item)}
                  onOpenInTab={onOpenChatInTab}
                  onDelete={onDeleteHistoryItem}
                  key={item.id}
                  historyItem={item}
                  disabled={item.id === currentChatId}
                />
              ))
            ) : (
              <Text as="p" size="2" mt="2">
                Your chat history is currently empty. Click &quot;New Chat&quot;
                to start a conversation.
              </Text>
            )}
          </Flex>
        </ScrollArea>
      </Box>
    );
  },
);

ChatHistory.displayName = "ChatHistory";
