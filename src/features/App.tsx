import React, { useEffect, useState } from "react";
import { useConfig } from "../contexts/config-context";
import { PageWrapper } from "../components/PageWrapper";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";
import { useLocalStorage } from "usehooks-ts";

export interface AppProps {
  style?: React.CSSProperties;
}

export const App: React.FC<AppProps> = ({ style }: AppProps) => {
  const { host } = useConfig();
  const { page, navigate } = usePages();
  const [apiKey, setApiKey] = useLocalStorage("api_key", "");
  const [loading, setLoading] = useState(false);

  const onPressNext = (host: Host) => {
    if (host === "cloud") {
      navigate({ type: "push", page: { name: "cloud login" } });
    } else if (host === "enterprise") {
      navigate({ type: "push", page: { name: "enterprise setup" } });
    } else {
      navigate({ type: "push", page: { name: "self hosting setup" } });
    }
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
    <PageWrapper host={host} style={style}>
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
          next={() => 0}
        />
      )}
      {page.name === "enterprise setup" && (
        <EnterpriseSetup goBack={goBack} next={() => 0} />
      )}
      {page.name === "self hosting setup" && (
        <SelfHostingSetup goBack={goBack} next={() => 0} />
      )}
    </PageWrapper>
  );
};
