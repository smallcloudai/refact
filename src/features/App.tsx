import React from "react";
import { useConfig } from "../contexts/config-context";
import { PageWrapper } from "../components/PageWrapper";
import { InitialSetup } from "../components/InitialSetup";

export const App: React.FC<{ style?: React.CSSProperties }> = ({ style }) => {
  const { host } = useConfig();

  const onPressNext = () => {
    return 0;
  };

  return (
    <PageWrapper host={host} style={style}>
      <InitialSetup onPressNext={onPressNext} />
    </PageWrapper>
  );
};
