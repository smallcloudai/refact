import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { useLocalStorage } from "usehooks-ts";
import { Flex } from "@radix-ui/themes";
// import { HistorySideBar } from "./HistorySideBar";
import { Chat } from "./Chat";
// import { Chat } from "../components/Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import {
  useEventBusForHost,
  usePostMessage,
  useChatHistory,
  useEventBusForChat,
  useEventBysForFIMDebug,
  useEventBusForStatistic,
} from "../hooks";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  SetupHost,
} from "../events/setup";
import { useConfig } from "../contexts/config-context";
import { FIMDebug } from "./FIMDebug";
import { Statistic } from "./Statistic";

export interface AppProps {
  style?: React.CSSProperties;
}

export const App: React.FC<AppProps> = ({ style }: AppProps) => {
  const { pages, navigate } = usePages();
  const [apiKey, setApiKey] = useLocalStorage("api_key", "");
  const { currentChatId } = useEventBusForHost();
  const config = useConfig();

  const postMessage = usePostMessage();

  const historyHook = useChatHistory();
  const chatHook = useEventBusForChat();
  const fimHook = useEventBysForFIMDebug();
  const statisticsHook = useEventBusForStatistic();

  const setupHost = useCallback(
    (host: HostSettings) => {
      const setupHost: SetupHost = {
        type: EVENT_NAMES_FROM_SETUP.SETUP_HOST,
        payload: {
          host,
        },
      };

      postMessage(setupHost);
    },
    [postMessage],
  );

  const onPressNext = (host: Host) => {
    if (host === "cloud") {
      navigate({ type: "push", page: { name: "cloud login" } });
    } else if (host === "enterprise") {
      navigate({ type: "push", page: { name: "enterprise setup" } });
    } else {
      navigate({ type: "push", page: { name: "self hosting setup" } });
    }
  };

  const cloudLogin = (apiKey: string, sendCorrectedCodeSnippets: boolean) => {
    setupHost({ type: "cloud", apiKey, sendCorrectedCodeSnippets });
    navigate({ type: "push", page: { name: "chat" } });
  };

  const enterpriseSetup = (apiKey: string, endpointAddress: string) => {
    setupHost({ type: "enterprise", apiKey, endpointAddress });
    navigate({ type: "push", page: { name: "chat" } });
  };

  const selfHostingSetup = (endpointAddress: string) => {
    setupHost({ type: "self", endpointAddress });
    navigate({ type: "push", page: { name: "history" } });
  };

  const openExternal = (url: string) => {
    window.open(url, "_blank")?.focus();
  };

  const goBack = () => {
    navigate({ type: "pop" });
  };

  const handleHistoryItemClick = useCallback(
    (id: string) => {
      const currentChat = historyHook.history.find((item) => item.id === id);
      if (currentChat) {
        historyHook.setCurrentChatId(id);
        historyHook.restoreChatFromHistory(id);
        // chatHook.restoreChat(currentChat);
        navigate({ type: "push", page: { name: "chat" } });
      }
    },
    [historyHook, navigate],
  );

  const handleCreateNewChat = useCallback(() => {
    historyHook.createNewChat();
    navigate({ type: "push", page: { name: "chat" } });
  }, [historyHook, navigate]);

  const handleDelete = useCallback(
    (id: string) => {
      historyHook.deleteChat(id);
    },
    [historyHook],
  );

  const handleNavigation = useCallback(
    (to: "fim" | "stats" | "hot keys" | "settings" | "") => {
      if (to === "settings") {
        chatHook.openSettings();
      } else if (to === "hot keys") {
        chatHook.openHotKeys();
      } else if (to === "fim") {
        navigate({
          type: "push",
          page: { name: "fill in the middle debug page" },
        });
      } else if (to === "stats") {
        navigate({ type: "push", page: { name: "statistics page" } });
      }
    },
    [chatHook, navigate],
  );

  // goTo settings, fim, stats, hot keys

  return (
    <Flex style={{ justifyContent: "center", ...style }}>
      {pages.map((page, i) => {
        return (
          <Flex key={i} display={i === pages.length - 1 ? "flex" : "none"}>
            {page.name === "initial setup" && (
              <InitialSetup onPressNext={onPressNext} />
            )}
            {page.name === "cloud login" && (
              <CloudLogin
                goBack={goBack}
                apiKey={apiKey}
                setApiKey={setApiKey}
                openExternal={openExternal}
                next={cloudLogin}
              />
            )}
            {page.name === "enterprise setup" && (
              <EnterpriseSetup goBack={goBack} next={enterpriseSetup} />
            )}
            {page.name === "self hosting setup" && (
              <SelfHostingSetup goBack={goBack} next={selfHostingSetup} />
            )}
            {page.name === "history" && (
              <Sidebar
                history={historyHook.history}
                takingNotes={false}
                currentChatId={currentChatId}
                onCreateNewChat={handleCreateNewChat}
                account={undefined}
                onHistoryItemClick={handleHistoryItemClick}
                onDeleteHistoryItem={handleDelete}
                onOpenChatInTab={undefined}
                handleLogout={() => {
                  // TODO: handle logout
                }}
                handleNavigation={handleNavigation}
              />
            )}
            {page.name === "chat" && (
              <Chat host={config.host} tabbed={config.tabbed} {...chatHook} />
            )}
            {page.name === "fill in the middle debug page" && (
              <FIMDebug
                state={fimHook.state}
                host={config.host}
                tabbed={config.tabbed}
                clearErrorMessage={fimHook.clearErrorMessage}
                backFromFim={goBack}
              />
            )}
            {page.name === "statistics page" && (
              <Statistic
                state={statisticsHook.state}
                backFromStatistic={goBack}
                tabbed={config.tabbed}
                host={config.host}
                onCloseStatistic={goBack}
              />
            )}
          </Flex>
        );
      })}
    </Flex>
  );
};
