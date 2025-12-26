import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export interface Welcome {
  name: "welcome";
}

export interface TourEnd {
  name: "tour end";
}

export interface HistoryList {
  name: "history";
}

export interface ChatPage {
  name: "chat";
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

export interface ChatThreadHistoryPage {
  name: "thread history page";
  // causes a bug with other pages
  chatId: string;
}

export interface LoginPage {
  name: "login page";
}

export interface ProvidersPage {
  name: "providers page";
}

export interface IntegrationsSetupPage {
  name: "integrations page";
  projectPath?: string;
  integrationName?: string;
  integrationPath?: string;
  shouldIntermediatePageShowUp?: boolean;
  wasOpenedThroughChat?: boolean;
}

export type Page =
  | ChatPage
  | Welcome
  | TourEnd
  | HistoryList
  | FIMDebugPage
  | StatisticsPage
  | DocumentationSettingsPage
  | ChatThreadHistoryPage
  | IntegrationsSetupPage
  | ProvidersPage
  | LoginPage;

export function isIntegrationSetupPage(
  page: Page,
): page is IntegrationsSetupPage {
  return page.name === "integrations page";
}

export type PageSliceState = Page[];

const initialState: PageSliceState = [{ name: "login page" }];

export const pagesSlice = createSlice({
  name: "pages",
  initialState,
  reducers: {
    pop: (state) => {
      state.pop();
    },
    push: (state, action: PayloadAction<Page>) => {
      state.push(action.payload);
    },
    popBackTo: (state, action: PayloadAction<Page>) => {
      const pageIndex = state.findIndex((page) => {
        if (
          isIntegrationSetupPage(action.payload) &&
          isIntegrationSetupPage(page) &&
          action.payload.projectPath === page.projectPath &&
          action.payload.integrationName === page.integrationName
        ) {
          return true;
        } else if (isIntegrationSetupPage(action.payload)) {
          return false;
        }
        return page.name === action.payload.name;
      });
      if (pageIndex === -1) {
        state.push(action.payload);
        return;
      }
      state.length = pageIndex + 1;
    },

    change: (state, action: PayloadAction<Page>) => {
      state.pop();
      state.push(action.payload);
    },
  },
  selectors: {
    isPageInHistory: (state, name: string) => {
      return state.some((page) => page.name === name);
    },
    selectPages: (state) => state,

    selectCurrentPage: (state) => {
      if (state.length === 0) return undefined;
      return state[state.length - 1];
    },
  },
});

export const { pop, push, popBackTo, change } = pagesSlice.actions;
export const { selectPages, isPageInHistory, selectCurrentPage } =
  pagesSlice.selectors;
