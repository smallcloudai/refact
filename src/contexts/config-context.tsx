import React, { useContext, createContext } from "react";
import { ThemeProps } from "../components/Theme";

export type Config = {
  rag?: boolean; // TODO: remove this
  host: "web" | "ide" | "vscode" | "jetbrains";
  tabbed?: boolean;
  lspUrl?: string;
  dev?: boolean;
  themeProps?: ThemeProps;
};

const ConfigContext = createContext<Config>({ host: "web" });

// TODO: add theme props, and configure vscode to grey
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
