import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { Flex } from "@radix-ui/themes";
import { Chat, newChatAction, selectChatId, selectIsStreaming } from "./Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { useEventsBusForIDE, useConfig, useEffectOnce } from "../hooks";

import { useAppSelector, useAppDispatch } from "../hooks";
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
import { TourProvider } from "./Tour";
import { Tour } from "../components/Tour";
import { TourEnd } from "../components/Tour/TourEnd";
import { useEventBusForApp } from "../hooks/useEventBusForApp";
import { BringYourOwnKey } from "../components/BringYourOwnKey/BringYourOwnKey";
import { AbortControllerProvider } from "../contexts/AbortControllers";
import { Toolbar } from "../components/Toolbar";
import { Tab } from "../components/Toolbar/Toolbar";
import { PageWrapper } from "../components/PageWrapper";
import { ThreadHistory } from "./ThreadHistory";
import { Integrations } from "./Integrations";
import { UserSurvey } from "./UserSurvey";
import { integrationsApi } from "../services/refact";

import styles from "./App.module.css";
import classNames from "classnames";

export interface AppProps {
  style?: React.CSSProperties;
}

export const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const dispatch = useAppDispatch();

  const pages = useAppSelector(selectPages);
  const isStreaming = useAppSelector(selectIsStreaming);

  const isPageInHistory = useCallback(
    (pageName: string) => {
      return pages.some((page) => page.name === pageName);
    },
    [pages],
  );

  const { setupHost, chatPageChange, setIsChatStreaming, setIsChatReady } =
    useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  const historyState = useAppSelector((state: RootState) => state.history);
  const chatId = useAppSelector(selectChatId);
  useEventBusForWeb();
  useEventBusForApp();

  const [isPaddingApplied, setIsPaddingApplied] = useState<boolean>(false);

  const handlePaddingShift = (state: boolean) => {
    setIsPaddingApplied(state);
  };

  const config = useConfig();

  const isLoggedIn =
    isPageInHistory("history") ||
    isPageInHistory("welcome") ||
    isPageInHistory("chat");

  useEffect(() => {
    if (config.apiKey && config.addressURL && !isLoggedIn) {
      if (tourState.type === "in_progress" && tourState.step === 1) {
        dispatch(push({ name: "welcome" }));
      } else if (Object.keys(historyState).length === 0) {
        dispatch(push({ name: "history" }));
        dispatch(newChatAction());
        dispatch(push({ name: "chat" }));
      } else {
        dispatch(push({ name: "history" }));
      }
    }
    if (!config.apiKey && !config.addressURL && isLoggedIn) {
      dispatch(popBackTo({ name: "initial setup" }));
    }
  }, [
    config.apiKey,
    config.addressURL,
    isLoggedIn,
    dispatch,
    tourState,
    historyState,
  ]);

  useEffect(() => {
    if (pages.length > 1) {
      const currentPage = pages.slice(-1)[0];
      chatPageChange(currentPage.name);
    }
  }, [pages, chatPageChange]);

  useEffect(() => {
    setIsChatStreaming(isStreaming);
  }, [isStreaming, setIsChatStreaming]);

  useEffectOnce(() => {
    setIsChatReady(true);
  });

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

  const goBackFromIntegrations = () => {
    dispatch(pop());
    dispatch(integrationsApi.util.resetApiState());
  };

  const page = pages[pages.length - 1];

  const activeTab: Tab | undefined = useMemo(() => {
    if (page.name === "chat") {
      return {
        type: "chat",
        id: chatId,
      };
    }
    if (page.name === "history") {
      return {
        type: "dashboard",
      };
    }
  }, [page, chatId]);

  return (
    <Flex
      align="stretch"
      direction="column"
      style={style}
      className={classNames(styles.rootFlex, {
        [styles.integrationsPagePadding]:
          page.name === "integrations page" && isPaddingApplied,
      })}
    >
      <PageWrapper
        host={config.host}
        style={{
          paddingRight: page.name === "integrations page" ? 0 : undefined,
        }}
      >
        <UserSurvey />
        {activeTab && <Toolbar activeTab={activeTab} />}
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
            style={{
              alignSelf: "stretch",
              height: "calc(100% - var(--space-5)* 2)",
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
        {page.name === "integrations page" && (
          <Integrations
            backFromIntegrations={goBackFromIntegrations}
            tabbed={config.tabbed}
            host={config.host}
            onCloseIntegrations={goBackFromIntegrations}
            handlePaddingShift={handlePaddingShift}
          />
        )}
        {page.name === "thread history page" && (
          <ThreadHistory
            backFromThreadHistory={goBack}
            tabbed={config.tabbed}
            host={config.host}
            onCloseThreadHistory={goBack}
            chatId={page.chatId}
          />
        )}
      </PageWrapper>
      <Tour page={pages[pages.length - 1].name} />
    </Flex>
  );
};

// TODO: move this to the `app` directory.
export const App = () => {
  return (
    <Provider store={store}>
      <PersistGate persistor={persistor}>
        <Theme>
          <TourProvider>
            <AbortControllerProvider>
              <InnerApp />
            </AbortControllerProvider>
          </TourProvider>
        </Theme>
      </PersistGate>
    </Provider>
  );
};
