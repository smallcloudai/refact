import React, { useEffect, useState } from "react";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { useLocalStorage } from "usehooks-ts";
import { Flex } from "@radix-ui/themes";
import { HistorySideBar } from "./HistorySideBar";
import { Chat } from "./Chat";
import { useEventBusForHost } from "../hooks";

export interface AppProps {
  style?: React.CSSProperties;
}

export const App: React.FC<AppProps> = ({ style }: AppProps) => {
  const { page, navigate } = usePages();
  const [apiKey, setApiKey] = useLocalStorage("api_key", "");
  const [loading, setLoading] = useState(false);
  const { takeingNotes, currentChatId } = useEventBusForHost();

  const onPressNext = (host: Host) => {
    if (host === "cloud") {
      navigate({ type: "push", page: { name: "cloud login" } });
    } else if (host === "enterprise") {
      navigate({ type: "push", page: { name: "enterprise setup" } });
    } else {
      navigate({ type: "push", page: { name: "self hosting setup" } });
    }
  };

  const goToChat = () => {
    navigate({ type: "push", page: { name: "chat" } });
  };

  const onLogin = () => {
    setLoading(true);
  };

  const goBack = () => {
    navigate({ type: "pop" });
  };

  useEffect(() => {
    setLoading(false);
  }, [apiKey]);

  return (
    <Flex style={{ justifyContent: "center", ...style }}>
      {page.name === "initial setup" && (
        <InitialSetup onPressNext={onPressNext} />
      )}
      {page.name === "cloud login" && (
        <CloudLogin
          goBack={goBack}
          loading={loading}
          apiKey={apiKey}
          setApiKey={setApiKey}
          login={onLogin}
          next={goToChat}
        />
      )}
      {page.name === "enterprise setup" && (
        <EnterpriseSetup goBack={goBack} next={goToChat} />
      )}
      {page.name === "self hosting setup" && (
        <SelfHostingSetup goBack={goBack} next={goToChat} />
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
};
