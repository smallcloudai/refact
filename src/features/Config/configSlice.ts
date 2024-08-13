import { createReducer } from "@reduxjs/toolkit";
import { createAction } from "@reduxjs/toolkit";
import { type ThemeProps } from "../../components/Theme";
import { RootState } from "../../app/store";

export type Config = {
  host: "web" | "ide" | "vscode" | "jetbrains";
  tabbed?: boolean;
  lspUrl?: string;
  dev?: boolean;
  // todo: handle light / darkmode
  themeProps: Omit<ThemeProps, "children">;
  features?: {
    statistics?: boolean;
    vecdb?: boolean;
    ast?: boolean;
  };
  apiKey?: string;
  addressURL?: string;
  lspPort: number;
};

// this could be taken from window.__INITAL_STATE
const initialState: Config = {
  host: "web",
  lspPort: 8001,
  features: {
    statistics: true,
    vecdb: true,
    ast: true,
  },
  themeProps: {
    appearance: "dark",
  },
};

export const updateConfig = createAction<Partial<Config>>("config/update");
export const setThemeMode = createAction<"light" | "dark" | "inherit">(
  "config/setThemeMode",
);

// TODO:
declare global {
  interface Window {
    __INITIAL_STATE?: Config;
  }
}

export const reducer = createReducer<Config>(
  () => window.__INITIAL_STATE ?? initialState,
  (builder) => {
    // TODO: toggle darkmode for web host?
    builder.addCase(updateConfig, (state, action) => {
      state.dev = action.payload.dev ?? state.dev;
      state.features = action.payload.features ?? state.features;
      state.host = action.payload.host ?? state.host;
      state.lspUrl = action.payload.lspUrl ?? state.lspUrl;
      state.tabbed = action.payload.tabbed ?? state.tabbed;
      state.themeProps = action.payload.themeProps ?? state.themeProps;
      state.apiKey = action.payload.apiKey ?? state.apiKey;
      state.addressURL = action.payload.addressURL ?? state.addressURL;
      state.lspPort = action.payload.lspPort ?? state.lspPort;
    });

    builder.addCase(setThemeMode, (state, action) => {
      state.themeProps.appearance = action.payload;
    });
  },
);

export const selectThemeMode = (state: RootState) =>
  state.config.themeProps.appearance;

export const selectConfig = (state: RootState) => state.config;
export const selectLspPort = (state: RootState) => state.config.lspPort;
