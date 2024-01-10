import React from "react";
import { useEventBusForHost } from "../../hooks/index.ts";
import { Theme } from "../../components/Theme/index.ts";
import { Flex } from "@radix-ui/themes";
import { HistorySideBar } from "../../features/HistorySideBar.tsx";
import { Chat } from "../../features/Chat.tsx";

export const ChatWithSideBar: React.FC = () => {
  useEventBusForHost();
  return (
    <Theme>
      <Flex>
        <HistorySideBar />
        <Chat style={{ maxWidth: "calc(100vw - 260px)" }} />
      </Flex>
    </Theme>
  );
};
