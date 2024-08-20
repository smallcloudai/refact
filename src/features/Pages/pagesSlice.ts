import { createSlice, PayloadAction } from "@reduxjs/toolkit";

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

export interface BringYourOwnKey {
  name: "bring your own key";
}

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

export type Page =
  | InitialSetupPage
  | CloudLogin
  | EnterpriseSetup
  | SelfHostingSetup
  | BringYourOwnKey
  | ChatPage
  | Welcome
  | TourEnd
  | HistoryList
  | FIMDebugPage
  | StatisticsPage
  | DocumentationSettingsPage;

export type PageSliceState = Page[];

const initialState: PageSliceState = [{ name: "initial setup" }];

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
    popBackTo: (state, action: PayloadAction<Page["name"]>) => {
      return state.slice(
        0,
        state.findIndex((page) => page.name === action.payload) + 1,
      );
    },

    change: (state, action: PayloadAction<Page>) => {
      const last = state.slice(0, -1);
      return last.concat(action.payload);
    },
  },
  selectors: {
    isPageInHistory: (state, name: string) => {
      return state.some((page) => page.name === name);
    },
    selectPages: (state) => state,
  },
});

export const { pop, push, popBackTo, change } = pagesSlice.actions;
export const { selectPages, isPageInHistory } = pagesSlice.selectors;
