import React from "react";
import { Box, Flex, Button } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Footer, FooterProps } from "./Footer";
import { Spinner } from "@radix-ui/themes";

export type SidebarProps = {
  onCreateNewChat: () => void;
  takingNotes: boolean;
  currentChatId: string;
  className?: string;
  style?: React.CSSProperties;
  account?: FooterProps["account"];
  handleLogout: () => void;
  handleNavigation: (
    to: "fim" | "stats" | "settings" | "hot keys" | "",
  ) => void;
} & ChatHistoryProps;

export const Sidebar: React.FC<SidebarProps> = ({
  history,
  onHistoryItemClick,
  onCreateNewChat,
  onDeleteHistoryItem,
  currentChatId,
  takingNotes,
  style,
  account,
  handleLogout,
  handleNavigation,
}) => {
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
  );
};
