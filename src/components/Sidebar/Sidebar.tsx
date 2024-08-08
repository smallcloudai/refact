import React from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import styles from "./sidebar.module.css";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Footer, FooterProps } from "./Footer";
import { Spinner } from "@radix-ui/themes";
import classNames from "classnames";
import { useAppSelector, useAppDispatch } from "../../app/hooks";
import {
  getHistory,
  deleteChatById,
} from "../../features/History/historySlice";
import { newChatAction, restoreChat } from "../../features/Chat2/chatThread";
import { ChatThread } from "../../events";

export type SidebarProps = {
  // onCreateNewChat: () => void;
  takingNotes: boolean;
  // currentChatId: string;
  className?: string;
  style?: React.CSSProperties;
  account?: FooterProps["account"];
  handleLogout: () => void;
  handleNavigation: (
    to: "fim" | "stats" | "settings" | "hot keys" | "chat" | "",
  ) => void;
} & Omit<
  ChatHistoryProps,
  | "history"
  | "onDeleteHistoryItem"
  | "onCreateNewChat"
  | "onHistoryItemClick"
  | "currentChatId"
>;

export const Sidebar: React.FC<SidebarProps> = ({
  // history,
  // onHistoryItemClick,
  // onCreateNewChat,
  // onDeleteHistoryItem,
  // currentChatId,
  takingNotes,
  className,
  style,
  account,
  handleLogout,
  handleNavigation,
}) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const history = useAppSelector(getHistory);
  const currentChatId = useAppSelector((state) => state.chat.thread.id);
  const onDeleteHistoryItem = (id: string) => dispatch(deleteChatById(id));
  const onCreateNewChat = () => {
    dispatch(newChatAction({ id: currentChatId }));
    handleNavigation("chat");
  };
  const onHistoryItemClick = (thread: ChatThread) =>
    dispatch(restoreChat({ id: currentChatId, thread }));

  return (
    <Box className={classNames(styles.sidebar, className)} style={style}>
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
          <Footer
            handleLogout={handleLogout}
            account={account}
            handleNavigation={handleNavigation}
          />
        </Flex>
      </Flex>
    </Box>
  );
};
