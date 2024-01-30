import React from "react";
import { Flex, Button } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";

export type ChatHistoryProps = {
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
  onCreateNewChat: () => void;
};

export const ChatHistory: React.FC<ChatHistoryProps> = ({
  history,
  onHistoryItemClick,
  onDeleteHistoryItem,
  onOpenChatInTab,
  onCreateNewChat,
}) => {
  return (
    <>
      <Flex mt="4" mb="4">
        <Button variant="outline" ml="auto" mr="auto" onClick={onCreateNewChat}>
          Start a new chat
        </Button>
      </Flex>
      <ScrollArea scrollbars="vertical">
        <Flex justify="center" align="center" pb="2" mr="1" direction="column">
          {history.map((chat) => (
            <HistoryItem
              onClick={onHistoryItemClick}
              onOpenInTab={onOpenChatInTab}
              onDelete={onDeleteHistoryItem}
              key={chat.id}
              chat={chat}
            />
          ))}
        </Flex>
      </ScrollArea>
    </>
  );
};
