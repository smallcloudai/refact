import React from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";

export const Sidebar: React.FC<
  {
    onCreateNewChat: () => void;
  } & ChatHistoryProps
> = ({ history, onHistoryItemClick, onCreateNewChat, onDeleteHistoryItem }) => {
  return (
    <Box className={styles.sidebar}>
      <Flex
        direction="column"
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
        <Box ml="auto" mr="auto" mt="4" mb="4">
          <Button variant="soft" onClick={onCreateNewChat}>
            Start a new chat
          </Button>
        </Box>

        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      </Flex>
    </Box>
  );
};
