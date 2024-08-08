import type { ThemeProps } from "../components/Theme";

export enum EVENT_NAMES_TO_CONFIG {
  UPDATE = "receive_config_update",
}

export type Config = {
  host: "web" | "ide" | "vscode" | "jetbrains";
  tabbed?: boolean;
  lspUrl?: string;
  dev?: boolean;
  themeProps?: ThemeProps;
  features?: {
    statistics?: boolean;
    vecdb?: boolean;
    ast?: boolean;
  };
  apiKey?: string;
  addressURL?: string;
};

export interface UpdateConfigMessage {
  type: EVENT_NAMES_TO_CONFIG.UPDATE;
  payload: Partial<Config>;
}

export function isUpdateConfigMessage(
  action: unknown,
): action is UpdateConfigMessage {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (action.type !== EVENT_NAMES_TO_CONFIG.UPDATE) return false;
  if (!("payload" in action)) return false;
  if (typeof action.payload !== "object") return false;
  return true;
}
