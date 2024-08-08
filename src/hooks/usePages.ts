import { useReducer } from "react";

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

export interface HistoryList {
  name: "history";
}

export interface FIMDebugPage {
  name: "fill in the middle debug page";
}

export interface StatisticsPage {
  name: "statistics page";
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
  | HistoryList
  | FIMDebugPage
  | StatisticsPage
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
  const firstPage: Page = { name: "initial setup" };
  const [pages, dispatch] = useReducer(pageReducer, [firstPage]);

  const isPageInHistory = (name: string) => {
    return pages.some((page) => page.name === name);
  };

  return { pages, navigate: dispatch, isPageInHistory };
}
