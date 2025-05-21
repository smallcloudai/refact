import React, { useCallback } from "react";
import { Box, Flex, Spinner } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  ChatHistoryItem,
  deleteChatById,
} from "../../features/History/historySlice";
import { push } from "../../features/Pages/pagesSlice";
import { restoreChat } from "../../features/Chat/Thread";
import { FeatureMenu } from "../../features/Config/FeatureMenu";
import { selectActiveGroup } from "../../features/Teams";
import { GroupTree } from "./GroupTree/";

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
  const maybeSelectedTeamsGroup = useAppSelector(selectActiveGroup);
  const history = useAppSelector((app) => app.history, {
    // TODO: selector issue here
    devModeChecks: { stabilityCheck: "never" },
  });

  const onDeleteHistoryItem = useCallback(
    (id: string) => dispatch(deleteChatById(id)),
    [dispatch],
  );

  const onHistoryItemClick = useCallback(
    (thread: ChatHistoryItem) => {
      dispatch(restoreChat(thread));
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  return (
    <Flex style={style}>
      <FeatureMenu />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>

      {maybeSelectedTeamsGroup ? (
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      ) : (
        <GroupTree />
      )}
    </Flex>
  );
};
