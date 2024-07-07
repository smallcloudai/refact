import React from "react";
import { useConfig } from "../contexts/config-context";
import { PageWrapper } from "../components/PageWrapper";
import { Host, InitialSetup } from "../components/InitialSetup";
import { usePages } from "../hooks/usePages";
import { CloudLogin } from "../components/CloudLogin";
import { EnterpriseSetup } from "../components/EnterpriseSetup";
import { SelfHostingSetup } from "../components/SelfHostingSetup";

export const App: React.FC<{ style?: React.CSSProperties }> = ({ style }) => {
  const { host } = useConfig();
  const { page, navigate } = usePages();

  const onPressNext = (host: Host) => {
    if (host === "cloud") {
      navigate({ type: "push", page: { name: "cloud login" } });
    } else if (host === "enterprise") {
      navigate({ type: "push", page: { name: "enterprise setup" } });
    } else {
      navigate({ type: "push", page: { name: "self hosting setup" } });
    }
  };

  const goBack = () => {
    navigate({ type: "pop" });
  };

  return (
    <PageWrapper host={host} style={style}>
      {page.name === "initial setup" && (
        <InitialSetup onPressNext={onPressNext} />
      )}
      {page.name === "cloud login" && (
        <CloudLogin
          goBack={goBack}
          loading={true}
          apiKey=""
          setApiKey={() => 0}
          login={() => 0}
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
