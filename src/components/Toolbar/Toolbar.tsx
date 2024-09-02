import { Button, Flex, Spinner, TabNav, Text } from "@radix-ui/themes";
import { Dropdown, DropdownNavigationOptions } from "./Dropdown";
import { DotFilledIcon, PlusIcon } from "@radix-ui/react-icons";
import { newChatAction } from "../../events";
import { restart, useTourRefs } from "../../features/Tour";
import { popBackTo, push } from "../../features/Pages/pagesSlice";
import { useCallback } from "react";
import { getHistory } from "../../features/History/historySlice";
import { restoreChat } from "../../features/Chat";
import { TruncateLeft } from "../Text";
import { useAppDispatch, useAppSelector } from "../../hooks";

export type DashboardTab = {
  type: "dashboard";
};

function isDashboardTab(tab: Tab): tab is DashboardTab {
  return tab.type === "dashboard";
}

export type ChatTab = {
  type: "chat";
  id: string;
};

function isChatTab(tab: Tab): tab is ChatTab {
  return tab.type === "chat";
}

export type Tab = DashboardTab | ChatTab;

export type ToolbarProps = {
  activeTab: Tab;
};

export const Toolbar = ({ activeTab }: ToolbarProps) => {
  const dispatch = useAppDispatch();

  const refs = useTourRefs();

  const history = useAppSelector(getHistory, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const isStreaming = useAppSelector((app) => app.chat.streaming);
  const cache = useAppSelector((app) => app.chat.cache);

  const handleNavigation = (to: DropdownNavigationOptions | "chat") => {
    if (to === "settings") {
      openSettings();
    } else if (to === "hot keys") {
      openHotKeys();
    } else if (to === "fim") {
      dispatch(push({ name: "fill in the middle debug page" }));
    } else if (to === "stats") {
      dispatch(push({ name: "statistics page" }));
    } else if (to === "restart tour") {
      dispatch(restart());
      dispatch(popBackTo("initial setup"));
      dispatch(push({ name: "welcome" }));
    } else if (to === "chat") {
      dispatch(popBackTo("history"));
      dispatch(push({ name: "chat" }));
    }
  };

  const onCreateNewChat = () => {
    dispatch(newChatAction());
    handleNavigation("chat");
  };

  const goToTab = useCallback(
    (tab: Tab) => {
      if (tab.type === "dashboard") {
        dispatch(popBackTo("history"));
        dispatch(newChatAction());
      } else {
        const chat = history.find((chat) => chat.id === tab.id);
        if (chat != undefined) {
          dispatch(restoreChat(chat));
        }
        dispatch(popBackTo("history"));
        dispatch(push({ name: "chat" }));
      }
    },
    [dispatch, history],
  );

  return (
    <Flex style={{ alignItems: "center", margin: 4, gap: 4 }}>
      <TabNav.Root style={{ flex: 1, overflowX: "scroll" }}>
        <TabNav.Link
          active={isDashboardTab(activeTab)}
          ref={(x) => refs.setBack(x)}
          onClick={() => goToTab({ type: "dashboard" })}
        >
          Dashboard
        </TabNav.Link>
        {history
          .filter(
            (chat) =>
              !chat.read ||
              (activeTab.type === "chat" && activeTab.id == chat.id),
          )
          .map((chat) => {
            const isStreamingThisTab =
              chat.id in cache ||
              (isChatTab(activeTab) && chat.id === activeTab.id && isStreaming);
            return (
              <TabNav.Link
                active={isChatTab(activeTab) && activeTab.id == chat.id}
                key={chat.id}
                onClick={() => goToTab({ type: "chat", id: chat.id })}
              >
                {isStreamingThisTab && <Spinner />}
                {!isStreamingThisTab && !chat.read && <DotFilledIcon />}
                <TruncateLeft style={{ maxWidth: "140px" }}>
                  {chat.title}
                </TruncateLeft>
              </TabNav.Link>
            );
          })}
      </TabNav.Root>
      <Button
        variant="outline"
        ref={(x) => refs.setNewChat(x)}
        onClick={onCreateNewChat}
      >
        <PlusIcon />
        <Text>New chat</Text>
      </Button>
      <Dropdown handleNavigation={handleNavigation} />
    </Flex>
  );
};

function openSettings() {
  throw new Error("Function not implemented.");
}

function openHotKeys() {
  throw new Error("Function not implemented.");
}
