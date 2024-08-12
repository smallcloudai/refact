import React, { useCallback } from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Footer, FooterProps } from "./Footer";
import { Spinner } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch } from "../../app/hooks";
import {
  getHistory,
  deleteChatById,
} from "../../features/History/historySlice";
import {
  newChatAction,
  restoreChat,
  selectChatId,
} from "../../features/Chat/chatThread";
import type { ChatThread } from "../../features/Chat/chatThread";

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
  style,
  account,
  handleLogout,
  handleNavigation,
}) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const history = useAppSelector(getHistory);

  const currentChatId = useAppSelector(selectChatId);
  const onDeleteHistoryItem = (id: string) => dispatch(deleteChatById(id));
  const onCreateNewChat = () => {
    dispatch(newChatAction({ id: currentChatId }));
    handleNavigation("chat");
  };
  const onHistoryItemClick = useCallback(
    (thread: ChatThread) => {
      dispatch(restoreChat({ id: currentChatId, thread }));
      handleNavigation("chat");
    },
    [currentChatId, dispatch, handleNavigation],
  );

  return (
    <Flex direction="column" style={style}>
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
      />
      <Flex p="2" pb="4">
        <Footer
          handleLogout={handleLogout}
          account={account}
          handleNavigation={handleNavigation}
        />
      </Flex>
    </Flex>
  );
};
