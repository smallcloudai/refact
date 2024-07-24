import { useReducer } from "react";
import { useConfig } from "../contexts/config-context";

export interface InitialSetupPage {
  name: "initial setup";
}

export interface CloudLogin {
  name: "cloud login";
}

export interface EnterpriseSetup {
  name: "enterprise setup";
}

export interface SelfHostingSetup {
  name: "self hosting setup";
}

export interface ChatPage {
  name: "chat";
}

export interface DocumentationSettingsPage {
  name: "documentation settings";
}

export type Page =
  | InitialSetupPage
  | CloudLogin
  | EnterpriseSetup
  | SelfHostingSetup
  | ChatPage
  | DocumentationSettingsPage;

export interface ChangePage {
  type: "change";
  page: Page;
}

export interface PushPage {
  type: "push";
  page: Page;
}

export interface PopPage {
  type: "pop";
}

export type PageAction = ChangePage | PushPage | PopPage;

function pageReducer(state: Page[], action: PageAction): Page[] {
  if (action.type === "pop") {
    return state.slice(0, -1);
  } else if (action.type === "change") {
    return [...state.slice(0, -1), action.page];
  } else {
    return [...state, action.page];
  }
}

export function usePages() {
  const config = useConfig();
  let firstPage: Page = { name: "documentation settings" };
  if (config.addressURL && config.apiKey) {
    firstPage = { name: "chat" }; // todo: change this to chat history
  }
  const [pages, dispatch] = useReducer(pageReducer, [firstPage]);

  return { pages, navigate: dispatch };
}
