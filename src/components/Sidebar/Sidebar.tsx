import React, { useState } from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Footer, FooterProps } from "./Footer";
import { Statistic } from "../../features/Statistic";
import { Spinner } from "@radix-ui/themes";
import classNames from "classnames";

export type SideBarProps = {
  onCreateNewChat: () => void;
  takingNotes: boolean;
  currentChatId: string;
  className?: string;
  style?: React.CSSProperties;
  account?: FooterProps["account"];
} & ChatHistoryProps;

export const Sidebar: React.FC<SideBarProps> = ({
  history,
  onHistoryItemClick,
  onCreateNewChat,
  onDeleteHistoryItem,
  currentChatId,
  takingNotes,
  className,
  style,
  account,
}) => {
  const [isOpenedStatistic, setIsOpenedStatistic] = useState(false);
  const handleCloseStatistic = () => {
    setIsOpenedStatistic(false);
  };
  // const { features } = useConfig();

  // const [currentItem, setItem] = useState("")

  return (
    <Box className={classNames(styles.sidebar, className)} style={style}>
      {isOpenedStatistic ? (
        <Statistic onCloseStatistic={handleCloseStatistic} />
      ) : (
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
            <Box position="absolute" ml="5" mt="2">
              <Spinner loading={takingNotes} title="taking notes" />
            </Box>
            <Button
              variant="outline"
              ml="auto"
              mr="auto"
              onClick={onCreateNewChat}
              // loading={takingNotes}
            >
              Start a new chat
            </Button>
          </Flex>
          <ChatHistory
            history={history}
            onHistoryItemClick={onHistoryItemClick}
            onDeleteHistoryItem={onDeleteHistoryItem}
            currentChatId={currentChatId}
          />
          <Flex p="2" pb="4">
            <Footer account={account} />
          </Flex>
        </Flex>
      )}
    </Box>
  );
};
