import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";
import { FTMMessage, makeMessageTrie } from "./makeMessageTrie";
import { pagesSlice } from "../Pages/pagesSlice";
import {
  createMessage,
  createThreadWithMessage,
} from "../../services/graphql/graphqlThunks";

type InitialState = {
  loading: boolean;
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

const ID_REGEXP = /^(.*):(\d+):(\d+):(\d+)$/;

function getInfoFromId(id: string) {
  const result = id.match(ID_REGEXP);
  if (result === null) return null;
  const [ftm_belongs_to_ft_id, ftm_alt, ftm_num, ftm_prev_alt] = result;
  return {
    ftm_belongs_to_ft_id,
    ftm_alt: +ftm_alt,
    ftm_num: +ftm_num,
    ftm_prev_alt: +ftm_prev_alt,
  };
}

export const threadMessagesSlice = createSlice({
  name: "threadMessages",
  initialState,
  reducers: {
    receiveThreadMessages: (
      state,
      action: PayloadAction<MessagesSubscriptionSubscription>,
    ) => {
      state.loading = false;
      // console.log(
      //   "receiveMessages",
      //   action.payload.comprehensive_thread_subs.news_action,
      //   action.payload,
      // );
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
        action.payload.comprehensive_thread_subs.news_action === "INSERT" &&
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

      if (
        action.payload.comprehensive_thread_subs.news_action === "DELTA" &&
        action.payload.comprehensive_thread_subs.stream_delta
      ) {
        if (
          action.payload.comprehensive_thread_subs.news_payload_id in
            state.messages &&
          action.payload.comprehensive_thread_subs
            .news_payload_thread_message &&
          "ftm_content" in action.payload.comprehensive_thread_subs.stream_delta
        ) {
          // TODO: handle deltas, delta don't have all the info though :/
          state.messages[
            action.payload.comprehensive_thread_subs.news_payload_id
          ].ftm_content +=
            action.payload.comprehensive_thread_subs.stream_delta.ftm_content;
        } else if (
          !(
            action.payload.comprehensive_thread_subs.news_payload_id in
            state.messages
          ) &&
          action.payload.comprehensive_thread_subs.news_payload_thread_message
        ) {
          const infoFromId = getInfoFromId(
            action.payload.comprehensive_thread_subs.news_payload_id,
          );

          const msg: FTMMessage = {
            // TODO: remove this, the key will suffice
            ...action.payload.comprehensive_thread_subs
              .news_payload_thread_message,
            ...infoFromId,
            ftm_role:
              action.payload.comprehensive_thread_subs.stream_delta.ftm_role,
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            ftm_content:
              action.payload.comprehensive_thread_subs.stream_delta.ftm_content,
            ftm_num:
              action.payload.comprehensive_thread_subs
                .news_payload_thread_message.ftm_num + 1,
          };
          state.messages[
            action.payload.comprehensive_thread_subs.news_payload_id
          ] = msg;
        }
      }
    },

    setThreadLeaf: (state, action: PayloadAction<InitialState["leaf"]>) => {
      state.leaf = action.payload;
    },

    resetThread: (state) => {
      state = initialState;
      return state;
    },

    setThreadFtId: (state, action: PayloadAction<InitialState["ft_id"]>) => {
      state.ft_id = action.payload;
    },
  },
  selectors: {
    selectThreadMessages: (state) => state.messages,
    selectThreadLoading: (state) => state.loading,
    selectThreadId: (state) => state.ft_id,
    selectThreadMessageTrie: (state) =>
      makeMessageTrie(Object.values(state.messages)),
    selectThreadLeaf: (state) => state.leaf,
    isThreadEmpty: (state) => Object.values(state.messages).length === 0,
  },

  extraReducers(builder) {
    builder.addCase(pagesSlice.actions.push, (state, action) => {
      if (
        action.payload.name === "chat" &&
        action.payload.ft_id !== state.ft_id
      ) {
        state = initialState;
      }
    });

    builder.addCase(createThreadWithMessage.pending, (state) => {
      state.loading = true;
    });
    builder.addCase(createThreadWithMessage.rejected, (state) => {
      state.loading = false;
    });
    builder.addCase(createMessage.pending, (state) => {
      state.loading = true;
    });
    builder.addCase(createMessage.rejected, (state) => {
      state.loading = false;
    });
  },
});

export const {
  receiveThreadMessages,
  setThreadLeaf,
  resetThread,
  setThreadFtId,
} = threadMessagesSlice.actions;
export const {
  selectThreadMessages,
  selectThreadLoading,
  selectThreadId,
  selectThreadMessageTrie,
  selectThreadLeaf,
  isThreadEmpty,
} = threadMessagesSlice.selectors;
