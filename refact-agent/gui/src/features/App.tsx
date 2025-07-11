import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Flex } from "@radix-ui/themes";
import { Chat } from "./Chat";
import { selectIsStreaming, selectThreadId } from "./ThreadMessages";
import { Sidebar } from "../components/Sidebar/Sidebar";
import {
  useAppSelector,
  useAppDispatch,
  useConfig,
  useEffectOnce,
  useEventsBusForIDE,
} from "../hooks";
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
import { Toolbar } from "../components/Toolbar";
import { Tab } from "../components/Toolbar/Toolbar";
import { PageWrapper } from "../components/PageWrapper";
import { ThreadHistory } from "./ThreadHistory";
import { Integrations } from "./Integrations";
import { Providers } from "./Providers";
import { UserSurvey } from "./UserSurvey";
import { integrationsApi } from "../services/refact";
import { LoginPage } from "./Login";

import styles from "./App.module.css";
import classNames from "classnames";
import { usePatchesAndDiffsEventsForIDE } from "../hooks/usePatchesAndDiffEventsForIDE";

import { selectActiveGroup } from "./Teams";
import { ConnectionStatus } from "./ConnectionStatus/ConectionStatus";

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

  const { chatPageChange, setIsChatStreaming, setIsChatReady } =
    useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  const maybeCurrentActiveGroup = useAppSelector(selectActiveGroup);
  const chatId = useAppSelector(selectThreadId);
  useEventBusForWeb();
  useEventBusForApp();
  usePatchesAndDiffsEventsForIDE();

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
      } else if (
        // threads length?
        maybeCurrentActiveGroup
      ) {
        dispatch(push({ name: "history" }));
        dispatch(push({ name: "chat" }));
      } else {
        dispatch(push({ name: "history" }));
      }
    }
    if (!config.apiKey && !config.addressURL && isLoggedIn) {
      dispatch(popBackTo({ name: "login page" }));
    }
  }, [
    config.apiKey,
    config.addressURL,
    isLoggedIn,
    dispatch,
    tourState,
    maybeCurrentActiveGroup,
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
        id: chatId ?? "",
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
        {page.name === "login page" && <LoginPage />}
        {activeTab && <Toolbar activeTab={activeTab} />}
        <ConnectionStatus />
        {page.name === "welcome" && <Welcome onPressNext={startTour} />}
        {page.name === "tour end" && <TourEnd />}
        {page.name === "history" && (
          <Sidebar
            takingNotes={false}
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
        {page.name === "providers page" && (
          <Providers
            backFromProviders={goBack}
            tabbed={config.tabbed}
            host={config.host}
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
      {page.name !== "welcome" && <Tour page={pages[pages.length - 1].name} />}
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
            <InnerApp />
          </TourProvider>
        </Theme>
      </PersistGate>
    </Provider>
  );
};
