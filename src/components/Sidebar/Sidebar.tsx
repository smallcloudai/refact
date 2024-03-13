import React, { useState } from "react";
import { Box, Flex, Button, IconButton } from "@radix-ui/themes";
import { BarChartIcon } from "@radix-ui/react-icons";
import styles from "./sidebar.module.css";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Settings } from "./Settings";
import { Statistic } from "../../features/Statistic";
import { useConfig } from "../../contexts/config-context";

export const Sidebar: React.FC<
  {
    onCreateNewChat: () => void;
  } & ChatHistoryProps
> = ({ history, onHistoryItemClick, onCreateNewChat, onDeleteHistoryItem }) => {
  const [isOpenedStatistic, setIsOpenedStatistic] = useState(false);
  const handleCloseStatistic = () => {
    setIsOpenedStatistic(false);
  };
  const { features } = useConfig();

  return (
    <Box className={styles.sidebar}>
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
          <Flex p="2" gap="1">
            <Settings />
            {features?.statistics && (
              <IconButton
                variant="outline"
                title="Bar Chart"
                onClick={() => setIsOpenedStatistic(true)}
              >
                <BarChartIcon />
              </IconButton>
            )}
          </Flex>
        </Flex>
      )}
    </Box>
  );
};
