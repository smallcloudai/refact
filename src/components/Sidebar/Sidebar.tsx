import React from "react";
import { Box, ScrollArea } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistoryItem } from "../../hooks/useChatHistory";
import { HistoryItem } from "./HistoryItem";

export const Sidebar: React.FC<{
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
}> = ({
  history,
  onHistoryItemClick
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
          {history.map((chat) => (
            <HistoryItem onClick={onHistoryItemClick} key={chat.id} chat={chat} />
          ))}
        </ScrollArea>
      </Box>
    </Box>
  );
};
