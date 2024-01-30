import React from "react";
import { Box, Flex } from "@radix-ui/themes";
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
        pr="2"
        pl="2"
        style={{
          // TODO: copy this to vscode
          zIndex: 1,
          overflowX: "hidden",
          width: "inherit",
        }}
      >
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
          onCreateNewChat={onCreateNewChat}
        />
      </Flex>
    </Box>
  );
};
