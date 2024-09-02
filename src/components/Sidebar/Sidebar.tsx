import React, { useCallback } from "react";
import { Box, Flex } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Spinner } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  getHistory,
  deleteChatById,
} from "../../features/History/historySlice";
import { Toolbar } from "../Toolbar";
import { push } from "../../features/Pages/pagesSlice";
import { PageWrapper } from "../PageWrapper";
import { useConfig } from "../../hooks";
import { restoreChat, type ChatThread } from "../../features/Chat/Thread";

export type SidebarProps = {
  takingNotes: boolean;
  className?: string;
  style?: React.CSSProperties;
} & Omit<
  ChatHistoryProps,
  | "history"
  | "onDeleteHistoryItem"
  | "onCreateNewChat"
  | "onHistoryItemClick"
  | "currentChatId"
>;

export const Sidebar: React.FC<SidebarProps> = ({ takingNotes, style }) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const history = useAppSelector(getHistory, {
    // TODO: selector issue here
    devModeChecks: { stabilityCheck: "never" },
  });

  const onDeleteHistoryItem = (id: string) => dispatch(deleteChatById(id));
  const onHistoryItemClick = useCallback(
    (thread: ChatThread) => {
      dispatch(restoreChat(thread));
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  const { host } = useConfig();

  return (
    <PageWrapper host={host} style={style}>
      <Toolbar activeTab={{ type: "dashboard" }} />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>
      <ChatHistory
        history={history}
        onHistoryItemClick={onHistoryItemClick}
        onDeleteHistoryItem={onDeleteHistoryItem}
      />
    </PageWrapper>
  );
};
