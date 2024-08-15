import React from "react";
import { useEventBusForHost } from "../../hooks/index.ts";
import { Theme } from "../../components/Theme/index.ts";
import { Flex } from "@radix-ui/themes";
import { HistorySideBar } from "../../features/HistorySideBar.tsx";
import { Chat } from "../../features/Chat.tsx";

export const ChatWithSideBar: React.FC = () => {
  const { takeingNotes, currentChatId } = useEventBusForHost();
  return (
    <Theme>
      <Flex>
        <HistorySideBar
          takingNotes={takeingNotes}
          currentChatId={currentChatId}
        />
        <Chat style={{ width: "calc(100vw - 260px)" }} />
      </Flex>
    </Theme>
  );
};
