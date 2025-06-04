import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";
import { FTMMessage, makeMessageTrie } from "./makeMessageTrie";

type InitialState = {
  loading: false;
  messages: Record<string, FTMMessage>;
  ft_id: string | null;
  leaf: FTMMessage | null;
};

const initialState: InitialState = {
  loading: false,
  messages: {},
  ft_id: null,
  leaf: null,
};

export const threadMessagesSlice = createSlice({
  name: "threadMessages",
  initialState,
  reducers: {
    receiveThreadMessages: (
      state,
      action: PayloadAction<MessagesSubscriptionSubscription>,
    ) => {
      state.loading = false;
      if (
        state.ft_id &&
        action.payload.comprehensive_thread_subs.news_payload_thread_message
          ?.ftm_belongs_to_ft_id !== state.ft_id
      ) {
        return state;
      }

      if (
        !state.ft_id &&
        action.payload.comprehensive_thread_subs.news_payload_thread?.ft_id
      ) {
        state.ft_id =
          action.payload.comprehensive_thread_subs.news_payload_thread.ft_id;
      }

      // TODO: are there other cases aside from update
      // actions: INITIAL_UPDATES_OVER | UPDATE | DELETE
      if (
        action.payload.comprehensive_thread_subs.news_action === "UPDATE" &&
        action.payload.comprehensive_thread_subs.news_payload_id &&
        action.payload.comprehensive_thread_subs.news_payload_thread_message
      ) {
        state.messages[
          action.payload.comprehensive_thread_subs.news_payload_id
        ] =
          action.payload.comprehensive_thread_subs.news_payload_thread_message;
      }

      if (
        action.payload.comprehensive_thread_subs.news_action === "DELETE" &&
        action.payload.comprehensive_thread_subs.news_payload_id
      ) {
        const msgs = Object.entries(state.messages).reduce<
          typeof state.messages
        >((acc, [key, value]) => {
          if (
            key === action.payload.comprehensive_thread_subs.news_payload_id
          ) {
            return acc;
          }
          return { ...acc, [key]: value };
        }, {});

        state.messages = msgs;
      }
    },

    setThreadLeaf: (state, action: PayloadAction<InitialState["leaf"]>) => {
      state.leaf = action.payload;
    },
  },
  selectors: {
    selectThreadMessages: (state) => state.messages,
    selectThreadLoading: (state) => state.loading,
    selectThreadId: (state) => state.ft_id,
    selectThreadMessageTrie: (state) =>
      makeMessageTrie(Object.values(state.messages)),
    selectThreadLeaf: (state) => state.leaf,
  },
});

export const { receiveThreadMessages, setThreadLeaf } =
  threadMessagesSlice.actions;
export const {
  selectThreadMessages,
  selectThreadLoading,
  selectThreadId,
  selectThreadMessageTrie,
  selectThreadLeaf,
} = threadMessagesSlice.selectors;
