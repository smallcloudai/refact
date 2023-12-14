import React from "react";
import { Box, ScrollArea, Flex, Button } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistoryItem } from "../../hooks/useChatHistory";
import { HistoryItem } from "./HistoryItem";

export const Sidebar: React.FC<{
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onCreateNewChat: () => void;
}> = ({
  history,
  onHistoryItemClick,
  onCreateNewChat,
}) => {
  return (
    <Box display={{ initial: "none", md: "block" }} className={styles.sidebar}>
      <Box
        position="fixed"
        left="0"
        bottom="0"
        top="0"
        style={{
          zIndex: 1,
          overflowX: "hidden",
          width: "inherit",
        }}
      >
        <ScrollArea scrollbars="vertical">
          <Flex style={{width: "240px"}} justify="center" align="center" pt="4" pb="4">
            <Button onClick={onCreateNewChat} style={{marginRight: "16px"}}>Start a new chat</Button>
          </Flex>
          {history.map((chat) => (
            <HistoryItem onClick={onHistoryItemClick} key={chat.id} chat={chat} />
          ))}
        </ScrollArea>
      </Box>
    </Box>
  );
};
