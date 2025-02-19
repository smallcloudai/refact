import React from "react";
import { Box, Flex } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Spinner } from "@radix-ui/themes";
import { FeatureMenu } from "../../features/Config/FeatureMenu";

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
  // const dispatch = useAppDispatch();
  // void dispatch(subscribeToThreadsThunk());
  // const history = useAppSelector((app) => app.history, {
  //   // TODO: selector issue here
  //   devModeChecks: { stabilityCheck: "never" },
  // });

  // const onDeleteHistoryItem = useCallback(
  //   (id: string) => dispatch(deleteChatById(id)),
  //   [dispatch],
  // );

  // const onHistoryItemClick = useCallback(
  //   (thread: ChatThread) => {
  //     dispatch(restoreChat(thread));
  //     dispatch(push({ name: "chat" }));
  //   },
  //   [dispatch],
  // );

  return (
    <Flex style={style}>
      <FeatureMenu />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>
      <ChatHistory />
    </Flex>
  );
};
