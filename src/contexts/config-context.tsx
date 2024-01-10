import React, { useContext, createContext } from "react";

export type Config = {
  vecdb: boolean;
  host: "web" | "ide"; // | "vscode" | "jetbrains"
  tabbed?: boolean;
  lspUrl?: string;
};

const ConfigContext = createContext<Config>({ vecdb: false, host: "web" });

const ConfigProvider: React.FC<{
  children: React.ReactNode;
  config: Config;
}> = ({ children, config }) => {
  return (
    <ConfigContext.Provider value={config}>{children}</ConfigContext.Provider>
  );
};

const useConfig = () => useContext(ConfigContext);

// eslint-disable-next-line react-refresh/only-export-components
export { ConfigProvider, useConfig };
