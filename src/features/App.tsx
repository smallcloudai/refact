import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { Flex } from "@radix-ui/themes";
import { Chat } from "./Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import {
  // useEventBusForHost,
  usePostMessage,
  // useChatHistory,
  // useEventBusForChat,
  useEventsBusForIDE,
} from "../hooks";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  OpenExternalUrl,
  SetupHost,
} from "../events/setup";
import { useConfig } from "../app/hooks";
import { FIMDebug } from "./FIM";
import { Statistics } from "./Statistics";
import { store, persistor } from "../app/store";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { Theme } from "../components/Theme";
import { useEventBusForApp } from "../hooks/useEventBusForApp";

export interface AppProps {
  style?: React.CSSProperties;
}

const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const { pages, navigate, isPageInHistory } = usePages();
  const { openHotKeys, openSettings } = useEventsBusForIDE();
  useEventBusForApp();
  // TODO: can replace this with a selector for state.chat.thread.id

  const postMessage = usePostMessage();
  const config = useConfig();

  // const historyHook = useChatHistory();
  // const chatHook = useEventBusForChat();
  // const fimHook = useEventBysForFIMDebug();
  // const statisticsHook = useEventBusForStatistic();

  if (config.apiKey && config.addressURL && !isPageInHistory("history")) {
    navigate({ type: "push", page: { name: "history" } });
  }

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
  };

  const enterpriseSetup = (apiKey: string, endpointAddress: string) => {
    setupHost({ type: "enterprise", apiKey, endpointAddress });
  };

  const selfHostingSetup = (endpointAddress: string) => {
    setupHost({ type: "self", endpointAddress });
  };

  const openExternal = (url: string) => {
    const openUrlMessage: OpenExternalUrl = {
      type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL,
      payload: { url },
    };
    postMessage(openUrlMessage);
  };

  const goBack = () => {
    navigate({ type: "pop" });
  };

  // const handleCreateNewChat = useCallback(() => {
  //   historyHook.createNewChat();
  //   navigate({ type: "push", page: { name: "chat" } });
  // }, [historyHook, navigate]);

  const handleNavigation = useCallback(
    (to: "fim" | "stats" | "hot keys" | "settings" | "chat" | "") => {
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
      } else if (to === "chat") {
        navigate({ type: "push", page: { name: "chat" } });
      }
    },
    [navigate, openHotKeys, openSettings],
  );

  // goTo settings, fim, stats, hot keys

  return (
    <Flex
      style={{
        flexDirection: "column",
        alignItems: "stretch",
        height: "100vh",
        ...style,
      }}
    >
      {pages.map((page, i) => {
        return (
          <Flex
            key={i}
            display={i === pages.length - 1 ? "flex" : "none"}
            style={{
              flexDirection: "row",
              height: "100%",
              justifyContent: "center",
            }}
          >
            {page.name === "initial setup" && (
              <InitialSetup onPressNext={onPressNext} />
            )}
            {page.name === "cloud login" && (
              <CloudLogin
                goBack={goBack}
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
                // history={historyHook.history}
                takingNotes={false}
                // currentChatId={currentChatId}
                // onCreateNewChat={handleCreateNewChat}
                account={undefined}
                // onHistoryItemClick={handleHistoryItemClick}
                // onDeleteHistoryItem={handleDelete}
                onOpenChatInTab={undefined}
                handleLogout={() => {
                  // TODO: handle logout
                }}
                handleNavigation={handleNavigation}
                style={{ maxWidth: "540px", flex: 1, height: "100%" }}
              />
            )}
            {page.name === "chat" && (
              <Chat
                host={config.host}
                tabbed={config.tabbed}
                // {...chatHook}
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
      <PersistGate persistor={persistor}>
        <Theme>
          <InnerApp />
        </Theme>
      </PersistGate>
    </Provider>
  );
};
