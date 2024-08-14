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

export interface Welcome {
  name: "welcome";
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
  | ChatPage
  | Welcome
  | HistoryList
  | FIMDebugPage
  | StatisticsPage
  | DocumentationSettingsPage;

// export interface ChangePage {
//   type: "change";
//   page: Page;
// }

// export interface PushPage {
//   type: "push";
//   page: Page;
// }

// export interface PopPage {
//   type: "pop";
// }

// export interface PopBackTo {
//   type: "pop_back_to";
//   page: Page["name"];
// }

// export type PageAction = ChangePage | PushPage | PopPage | PopBackTo;

// function pageReducer(state: Page[], action: PageAction): Page[] {
//   if (action.type === "pop") {
//     return state.slice(0, -1);
//   } else if (action.type === "change") {
//     return [...state.slice(0, -1), action.page];
//   } else if (action.type === "pop_back_to") {
//     return state.slice(
//       0,
//       state.findIndex((page) => page.name === action.page) + 1,
//     );
//   } else {
//     return [...state, action.page];
//   }
// }

// export function usePages() {
//   const firstPage: Page = { name: "initial setup" };
//   const [pages, dispatch] = useReducer(pageReducer, [firstPage]);

//   const isPageInHistory = (name: string) => {
//     return pages.some((page) => page.name === name);
//   };

//   return { pages, navigate: dispatch, isPageInHistory };
// }

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
