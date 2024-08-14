import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
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
import { useAppDispatch, useAppSelector, useConfig } from "../app/hooks";
import { FIMDebug } from "./FIM";
import { store, persistor, RootState } from "../app/store";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { Theme } from "../components/Theme";
import { useEventBusForApp } from "../hooks/useEventBusForApp";
import { Statistics } from "./statistics";
import { Welcome } from "../components/Tour";
import {
  push,
  popBackTo,
  pop,
  selectPages,
} from "../features/Pages/pagesSlice";
import { TourProvider, restart } from "./Tour";
import { Tour } from "../components/Tour";
import { DropdownNavigationOptions } from "../components/Sidebar/Footer";
import { TourEnd } from "../components/Tour/TourEnd";

export interface AppProps {
  style?: React.CSSProperties;
}

const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const dispatch = useAppDispatch();
  const pages = useAppSelector(selectPages);
  const isPageInHistory = useCallback(
    (pageName: string) => {
      return pages.some((page) => page.name === pageName);
    },
    [pages],
  );

  const { openHotKeys, openSettings } = useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  useEventBusForApp();
  // TODO: can replace this with a selector for state.chat.thread.id

  const postMessage = usePostMessage();
  const config = useConfig();

  const isLoggedIn = isPageInHistory("history") || isPageInHistory("welcome");

  if (config.apiKey && config.addressURL && !isLoggedIn) {
    if (tourState.type === "in_progress" && tourState.step === 1) {
      dispatch(push({ name: "welcome" }));
    } else {
      dispatch(push({ name: "history" }));
    }
  }
  if (!config.apiKey && !config.addressURL && isLoggedIn) {
    dispatch(popBackTo("initial setup"));
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
      dispatch(push({ name: "cloud login" }));
    } else if (host === "enterprise") {
      dispatch(push({ name: "enterprise setup" }));
    } else {
      dispatch(push({ name: "self hosting setup" }));
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
    dispatch(push({ name: "history" }));
  };

  const openExternal = (url: string) => {
    const openUrlMessage: OpenExternalUrl = {
      type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL,
      payload: { url },
    };
    postMessage(openUrlMessage);
  };

  const goBack = () => {
    dispatch(pop());
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
        dispatch(push({ name: "fill in the middle debug page" }));
      } else if (to === "stats") {
        dispatch(push({ name: "statistics page" }));
      } else if (to === "restart tour") {
        dispatch(restart());
        dispatch(popBackTo("initial setup"));
        dispatch(push({ name: "welcome" }));
      } else if (to === "chat") {
        dispatch(push({ name: "chat" }));
      }
    },
    [dispatch, openHotKeys, openSettings],
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
            {page.name === "tour end" && <TourEnd />}
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
      <Tour page={pages[pages.length - 1].name} />
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
