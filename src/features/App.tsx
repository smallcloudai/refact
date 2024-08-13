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
import { useAppSelector, useConfig } from "../app/hooks";
import { FIMDebug } from "./FIM";
import { store, persistor, RootState } from "../app/store";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { Theme } from "../components/Theme";
import { useEventBusForApp } from "../hooks/useEventBusForApp";
import { Statistics } from "./statistics";
import { Welcome } from "../components/Tour";
import { TourProvider } from "./Tour";
import { Tour } from "../components/Tour/Tour";
import { DropdownNavigationOptions } from "../components/Sidebar/Footer";

export interface AppProps {
  style?: React.CSSProperties;
}

const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const { pages, navigate, isPageInHistory } = usePages();
  const { openHotKeys, openSettings } = useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  useEventBusForApp();
  // TODO: can replace this with a selector for state.chat.thread.id

  const postMessage = usePostMessage();
  const config = useConfig();

  // const historyHook = useChatHistory();
  // const chatHook = useEventBusForChat();
  // const fimHook = useEventBysForFIMDebug();
  // const statisticsHook = useEventBusForStatistic();

  const isLoggedIn = isPageInHistory("history") || isPageInHistory("welcome");

  if (config.apiKey && config.addressURL && !isLoggedIn) {
    if (tourState.type === "in_progress" && tourState.step === 1) {
      navigate({ type: "push", page: { name: "welcome" } });
    } else {
      navigate({ type: "push", page: { name: "history" } });
    }
  }
  if (!config.apiKey && !config.addressURL && isLoggedIn) {
    navigate({ type: "pop_back_to", page: "initial setup" });
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

  const logOut = () => {
    postMessage({ type: EVENT_NAMES_FROM_SETUP.LOG_OUT });
  };

  const startTour = () => {
    navigate({ type: "push", page: { name: "history" } });
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
    (to: DropdownNavigationOptions | "chat") => {
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
            {page.name === "welcome" && <Welcome onPressNext={startTour} />}
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
                handleLogout={logOut}
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
      <Tour page={pages[pages.length - 1].name} navigate={navigate} />
    </Flex>
  );
};

export const App = () => {
  return (
    <Provider store={store}>
      <PersistGate persistor={persistor}>
        <Theme>
          <TourProvider>
            <InnerApp />
          </TourProvider>
        </Theme>
      </PersistGate>
    </Provider>
  );
};
