import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { ThreadsPageSubsSubscription } from "../../../generated/documents";
import { errorSlice } from "../Errors/errorsSlice";
import {
  deleteThreadThunk,
  threadsPageSub,
} from "../../services/graphql/graphqlThunks";

export type ThreadListItem = Exclude<
  ThreadsPageSubsSubscription["threads_in_group"]["news_payload"],
  undefined | null
>;

export type InitialState = {
  threads: Record<string, ThreadListItem>;
  loading: boolean;
  error: string | null;
  deleting: string[];
};

const initialState: InitialState = {
  threads: {},
  loading: false,
  error: null,
  deleting: [],
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
      // TBD: other data about the subscription

      const { news_action, news_payload } = action.payload.threads_in_group;
      if (news_action === "INITIAL_UPDATES_OVER") {
        state.loading = false;
      }

      if (!news_payload) return;

      // state.error = news_payload.ft_error || null;

      if (news_action === "UPDATE") {
        state.threads[news_payload.ft_id] = news_payload;
      }

      if (news_action === "DELETE") {
        state.threads = Object.entries(state.threads).reduce<
          InitialState["threads"]
        >((acc, [key, value]) => {
          if (key === news_payload.ft_id) return acc;
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
    selectThreadList: (state) => {
      return Object.values(state.threads);
    },

    selectThreadListError: (state) => state.error,

    selectThreadListState: (state) => state,

    selectThreadListLoading: (state) => state.loading,

    selectThreadIsDeleting: (state, id: string) => state.deleting.includes(id),
  },

  extraReducers(builder) {
    // TODO: add this for error slice?
    builder.addCase(errorSlice.actions.clearError, (state) => {
      state.error = null;
    });

    builder.addCase(deleteThreadThunk.pending, (state, action) => {
      state.deleting.push(action.meta.arg.id);
    });

    builder.addCase(deleteThreadThunk.fulfilled, (state, action) => {
      state.deleting = state.deleting.filter((id) => id !== action.payload.id);
    });

    builder.addCase(threadsPageSub.pending, (state) => {
      state.loading = true;
    });
  },
});

export const {
  selectThreadList,
  selectThreadListError,
  selectThreadListState,
  selectThreadListLoading,
  selectThreadIsDeleting,
} = threadListSlice.selectors;

export const {
  handleThreadListSubscriptionData,
  clearThreadListError,
  setThreadListError,
} = threadListSlice.actions;
