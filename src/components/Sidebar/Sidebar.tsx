import React from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistoryItem } from "../../hooks/useChatHistory";
import { HistoryItem } from "./HistoryItem";
import { ScrollArea } from "../ScrollArea";

export const Sidebar: React.FC<{
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onCreateNewChat: () => void;
}> = ({ history, onHistoryItemClick, onCreateNewChat }) => {
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
          <Flex
            justify="center"
            align="center"
            pt="4"
            pb="2"
            mr="1"
            direction="column"
            style={{
              backgroundColor: "var(--color-autofill-root)",
            }}
          >
            <Button
              variant="soft"
              onClick={onCreateNewChat}
              style={{
                marginBottom: "16px",
              }}
            >
              Start a new chat
            </Button>
            {history.map((chat) => (
              <HistoryItem
                onClick={onHistoryItemClick}
                key={chat.id}
                chat={chat}
              />
            ))}
          </Flex>
        </ScrollArea>
      </Box>
    </Box>
  );
};
