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
          width: "inherit",
        }}
      >
        <Flex mt="4" mb="4">
          <Button
            variant="outline"
            ml="auto"
            mr="auto"
            onClick={onCreateNewChat}
          >
            Start a new chat
          </Button>
        </Flex>
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      </Flex>
    </Box>
  );
};
