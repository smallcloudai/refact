import { createReducer } from "@reduxjs/toolkit";
import { createAction } from "@reduxjs/toolkit";
import { type ThemeProps } from "../../components/Theme";
import { RootState } from "../../app/store";

export type Config = {
  host: "web" | "ide" | "vscode" | "jetbrains";
  lspPort: number;
  tabbed?: boolean;
  lspUrl?: string;
  dev?: boolean;
  // todo: handle light / darkmode
  themeProps: Omit<ThemeProps, "children">;
  features?: {
    statistics?: boolean;
    vecdb?: boolean;
    ast?: boolean;
    images?: boolean;
  };
  keyBindings?: {
    completeManual?: string;
  };
  apiKey?: string | null;
  addressURL?: string;
  shiftEnterToSubmit?: boolean;
};

const initialState: Config = {
  host: "web",
  lspPort: __REFACT_LSP_PORT__ ?? 8001,
  apiKey: null,
  features: {
    statistics: true,
    vecdb: true,
    ast: true,
    images: true,
  },
  themeProps: {
    appearance: "dark",
  },
  shiftEnterToSubmit: false,
};

export const updateConfig = createAction<Partial<Config>>("config/update");
export const setThemeMode = createAction<"light" | "dark" | "inherit">(
  "config/setThemeMode",
);
export const setApiKey = createAction<string | null>("config/setApiKey");

export const reducer = createReducer<Config>(initialState, (builder) => {
  // TODO: toggle darkmode for web host?
  builder.addCase(updateConfig, (state, action) => {
    state.dev = action.payload.dev ?? state.dev;

    state.features = action.payload.features
      ? { ...state.features, ...action.payload.features }
      : state.features;

    state.host = action.payload.host ?? state.host;
    state.lspUrl = action.payload.lspUrl ?? state.lspUrl;
    state.tabbed = action.payload.tabbed ?? state.tabbed;
    state.themeProps = action.payload.themeProps ?? state.themeProps;
    state.apiKey = action.payload.apiKey ?? state.apiKey;
    state.addressURL = action.payload.addressURL ?? state.addressURL;
    state.lspPort = action.payload.lspPort ?? state.lspPort;
    state.keyBindings = action.payload.keyBindings ?? state.keyBindings;
    state.shiftEnterToSubmit =
      action.payload.shiftEnterToSubmit ?? state.shiftEnterToSubmit;
  });

  builder.addCase(setThemeMode, (state, action) => {
    state.themeProps.appearance = action.payload;
  });

  builder.addCase(setApiKey, (state, action) => {
    state.apiKey = action.payload;
  });
});

export const selectThemeMode = (state: RootState) =>
  state.config.themeProps.appearance;

export const selectConfig = (state: RootState) => state.config;
export const selectLspPort = (state: RootState) => state.config.lspPort;
export const selectVecdb = (state: RootState) =>
  state.config.features?.vecdb ?? false;
export const selectAst = (state: RootState) =>
  state.config.features?.ast ?? false;

export const selectApiKey = (state: RootState) => state.config.apiKey;
export const selectAddressURL = (state: RootState) => state.config.addressURL;
export const selectHost = (state: RootState) => state.config.host;
export const selectSubmitOption = (state: RootState) =>
  state.config.shiftEnterToSubmit ?? false;
