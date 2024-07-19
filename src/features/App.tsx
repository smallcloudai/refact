import React, { useCallback } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { useLocalStorage } from "usehooks-ts";
import { Flex } from "@radix-ui/themes";
import { HistorySideBar } from "./HistorySideBar";
import { Chat } from "./Chat";
import { useEventBusForHost, usePostMessage } from "../hooks";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  OpenExternalUrl,
  SetupHost,
} from "../events/setup";

export interface AppProps {
  style?: React.CSSProperties;
}

export const App: React.FC<AppProps> = ({ style }: AppProps) => {
  const { pages, navigate } = usePages();
  const [apiKey, setApiKey] = useLocalStorage("api_key", "");
  const { takeingNotes, currentChatId } = useEventBusForHost();
  const postMessage = usePostMessage();

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
    navigate({ type: "push", page: { name: "chat" } });
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
            {page.name === "chat" && (
              <>
                <HistorySideBar
                  takingNotes={takeingNotes}
                  currentChatId={currentChatId}
                />
                <Chat style={{ width: "calc(100vw - 260px)" }} />
              </>
            )}
          </Flex>
        );
      })}
    </Flex>
  );
};
