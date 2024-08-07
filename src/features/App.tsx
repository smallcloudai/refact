import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { useLocalStorage } from "usehooks-ts";
import { Flex } from "@radix-ui/themes";
import { Chat } from "./Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import {
  useEventBusForHost,
  usePostMessage,
  useChatHistory,
  useEventBusForChat,
  useEventsBusForIDE,
} from "../hooks";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  SetupHost,
} from "../events/setup";
import { useConfig } from "../app/hooks";
import { FIMDebug } from "./FIM";
import { Statistics } from "./Statistics";
import { store } from "../app/store";
import { Provider } from "react-redux";
import { Theme } from "../components/Theme";

export interface AppProps {
  style?: React.CSSProperties;
}

// TODO: wrap this in the Prvider and theme components
const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const { pages, navigate } = usePages();
  const { openHotKeys, openSettings } = useEventsBusForIDE();
  const [apiKey, setApiKey] = useLocalStorage("api_key", "");
  // TODO: can replace this with a selector for state.chat.thread.id
  const { currentChatId } = useEventBusForHost();
  const config = useConfig();

  const postMessage = usePostMessage();

  const historyHook = useChatHistory();
  const chatHook = useEventBusForChat();
  // const fimHook = useEventBysForFIMDebug();
  // const statisticsHook = useEventBusForStatistic();

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
        openSettings();
      } else if (to === "hot keys") {
        openHotKeys();
      } else if (to === "fim") {
        navigate({
          type: "push",
          page: { name: "fill in the middle debug page" },
        });
      } else if (to === "stats") {
        navigate({ type: "push", page: { name: "statistics page" } });
      }
    },
    [navigate, openHotKeys, openSettings],
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
              <Chat
                host={config.host}
                tabbed={config.tabbed}
                {...chatHook}
                backFromChat={goBack}
              />
            )}
            {page.name === "fill in the middle debug page" && (
              <FIMDebug host={config.host} tabbed={config.tabbed} />
            )}
            {page.name === "statistics page" && (
              <Statistics
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

export const App = () => {
  return (
    <Provider store={store}>
      <Theme>
        <InnerApp />
      </Theme>
    </Provider>
  );
};
