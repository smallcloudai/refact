import React from "react";
import { Flex } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";

export type ChatHistoryProps = {
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onDeleteHistoryItem: (id: string) => void;
};

export const ChatHistory: React.FC<ChatHistoryProps> = ({
  history,
  onHistoryItemClick,
  onDeleteHistoryItem,
}) => {
  return (
    <ScrollArea scrollbars="vertical">
      <Flex justify="center" align="center" pb="2" mr="1" direction="column">
        {history.map((chat) => (
          <HistoryItem
            onClick={onHistoryItemClick}
            onDelete={onDeleteHistoryItem}
            key={chat.id}
            chat={chat}
          />
        ))}
      </Flex>
    </ScrollArea>
  );
};
