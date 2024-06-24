import React from "react";
import { Flex, Box } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";

export type ChatHistoryProps = {
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
  currentChatId?: string;
};

export const ChatHistory: React.FC<ChatHistoryProps> = ({
  history,
  onHistoryItemClick,
  onDeleteHistoryItem,
  onOpenChatInTab,
  currentChatId,
}) => {
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
          {history.map((chat) => (
            <HistoryItem
              onClick={onHistoryItemClick}
              onOpenInTab={onOpenChatInTab}
              onDelete={onDeleteHistoryItem}
              key={chat.id}
              chat={chat}
              disabled={chat.id === currentChatId}
            />
          ))}
        </Flex>
      </ScrollArea>
    </Box>
  );
};
