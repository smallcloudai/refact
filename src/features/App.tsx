import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { Flex } from "@radix-ui/themes";
import { Chat } from "./Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { useEventsBusForIDE, useConfig } from "../hooks";

import { useAppDispatch, useAppSelector } from "../app/hooks";
import { FIMDebug } from "./FIM";
import { store, persistor, RootState } from "../app/store";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { Theme } from "../components/Theme";
import { useEventBusForWeb } from "../hooks/useEventBusForWeb";
import { Statistics } from "./Statistics";
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
import { useEventBusForApp } from "../hooks/useEventBusForApp";
import { BringYourOwnKey } from "../components/BringYourOwnKey/BringYourOwnKey";

export interface AppProps {
  style?: React.CSSProperties;
}

export const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const dispatch = useAppDispatch();
  const pages = useAppSelector(selectPages);
  const isPageInHistory = useCallback(
    (pageName: string) => {
      return pages.some((page) => page.name === pageName);
    },
    [pages],
  );

  const { openHotKeys, openSettings, setupHost } = useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  useEventBusForWeb();
  useEventBusForApp();

  const config = useConfig();

  const isLoggedIn =
    isPageInHistory("history") ||
    isPageInHistory("welcome") ||
    isPageInHistory("chat");

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

  const onPressNext = (host: Host) => {
    if (host === "cloud") {
      dispatch(push({ name: "cloud login" }));
    } else if (host === "enterprise") {
      dispatch(push({ name: "enterprise setup" }));
    } else if (host === "self-hosting") {
      dispatch(push({ name: "self hosting setup" }));
    } else {
      dispatch(push({ name: "bring your own key" }));
    }
  };

  const enterpriseSetup = (endpointAddress: string, apiKey: string) => {
    setupHost({ type: "enterprise", apiKey, endpointAddress });
  };

  const selfHostingSetup = (endpointAddress: string) => {
    setupHost({ type: "self", endpointAddress });
  };

  const bringYourOwnKeySetup = () => {
    setupHost({ type: "bring-your-own-key" });
  };

  const startTour = () => {
    dispatch(push({ name: "history" }));
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
            {page.name === "cloud login" && <CloudLogin goBack={goBack} />}
            {page.name === "enterprise setup" && (
              <EnterpriseSetup goBack={goBack} next={enterpriseSetup} />
            )}
            {page.name === "self hosting setup" && (
              <SelfHostingSetup goBack={goBack} next={selfHostingSetup} />
            )}
            {page.name === "bring your own key" && (
              <BringYourOwnKey goBack={goBack} next={bringYourOwnKeySetup} />
            )}
            {page.name === "welcome" && <Welcome onPressNext={startTour} />}
            {page.name === "tour end" && <TourEnd />}
            {page.name === "history" && (
              <Sidebar
                takingNotes={false}
                onOpenChatInTab={undefined}
                handleNavigation={handleNavigation}
                style={{
                  maxWidth: "min(100vw, 540px)",
                  flex: 1,
                  height: "100%",
                }}
              />
            )}
            {page.name === "chat" && (
              <Chat
                host={config.host}
                tabbed={config.tabbed}
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
