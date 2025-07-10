import {
  createSelector,
  createSlice,
  type PayloadAction,
} from "@reduxjs/toolkit";
import { ThreadsPageSubsSubscription } from "../../../generated/documents";
import { errorSlice } from "../Errors/errorsSlice";

export type ThreadListItem = Exclude<
  ThreadsPageSubsSubscription["threads_in_group"]["news_payload"],
  undefined | null
>;

export type InitialState = {
  threads: Record<string, ThreadListItem>;
  loading: boolean;
  error: string | null;
};

const initialState: InitialState = {
  threads: {},
  loading: false,
  error: null,
};

// type NewsAction = "UPDATE" | "DELETE" | "INITIAL_UPDATES_OVER";

export const threadListSlice = createSlice({
  name: "threadList",
  initialState,
  reducers: {
    handleThreadListSubscriptionData: (
      state,
      action: PayloadAction<ThreadsPageSubsSubscription>,
    ) => {
      const { news_action, news_payload, news_payload_id } =
        action.payload.threads_in_group;
      if (news_action === "INITIAL_UPDATES_OVER") {
        state.loading = false;
      }

      if (news_action === "UPDATE" && news_payload) {
        state.threads[news_payload.ft_id] = news_payload;
      }

      if (news_action === "DELETE" && news_payload_id) {
        state.threads = Object.entries(state.threads).reduce<
          InitialState["threads"]
        >((acc, [key, value]) => {
          if (key === news_payload_id) return acc;
          acc[key] = value;
          return acc;
        }, {});
      }
    },

    clearThreadListError: (state) => {
      state.error = null;
    },
    setThreadListError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },

    setThreadListLoading: (state, action: PayloadAction<boolean>) => {
      state.loading = action.payload;
    },
  },

  selectors: {
    // selectThreadList: (state) => {
    //   return Object.values(state.threads);
    // },
    selectThreadList: createSelector(
      (state: InitialState) => state.threads,
      (threads) =>
        Object.values(threads).sort(
          (a, b) => b.ft_updated_ts - a.ft_updated_ts,
        ),
    ),

    selectThreadListError: (state) => state.error,

    selectThreadListState: (state) => state,

    selectThreadListLoading: (state) => state.loading,
  },

  extraReducers(builder) {
    // TODO: add this for error slice?
    builder.addCase(errorSlice.actions.clearError, (state) => {
      state.error = null;
    });
  },
});

export const {
  selectThreadList,
  selectThreadListError,
  selectThreadListState,
  selectThreadListLoading,
} = threadListSlice.selectors;

export const {
  handleThreadListSubscriptionData,
  clearThreadListError,
  setThreadListError,
} = threadListSlice.actions;
