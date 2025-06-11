import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";
import {
  FTMMessage,
  makeMessageTrie,
  getPathToEndNode,
} from "./makeMessageTrie";
import { pagesSlice } from "../Pages/pagesSlice";
import {
  createMessage,
  createThreadWithMessage,
} from "../../services/graphql/graphqlThunks";

type InitialState = {
  isWaiting: boolean;
  isStreaming: boolean;
  messages: Record<string, FTMMessage>;
  ft_id: string | null;
  endNumber: number;
  endAlt: number;
  endPrevAlt: number;
};

const initialState: InitialState = {
  isWaiting: false,
  isStreaming: false,
  messages: {},
  ft_id: null,
  endNumber: 0,
  endAlt: 0,
  endPrevAlt: 0,
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
      state.isWaiting = false;
      // state.isStreaming = true; // TODO: figure out how to tell when the stream has ended
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

    setThreadEnd: (
      state,
      action: PayloadAction<{ number: number; alt: number; prevAlt: number }>,
    ) => {
      state.endNumber = action.payload.number;
      state.endAlt = action.payload.alt;
      state.endPrevAlt = action.payload.prevAlt;
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
    selectThreadId: (state) => state.ft_id,
    selectIsWaiting: (state) => state.isWaiting,
    selectIsStreaming: (state) => state.isStreaming,
    selectIsWaitingOrStreaming: (state) => state.isStreaming || state.isWaiting,
    selectThreadMessageTrie: (state) =>
      makeMessageTrie(Object.values(state.messages)),
    selectThreadEnd: (state) => {
      const { endNumber, endAlt, endPrevAlt } = state;
      return { endNumber, endAlt, endPrevAlt };
    },
    isThreadEmpty: (state) => Object.values(state.messages).length === 0,
    selectAppSpecific: (state) => {
      const values = Object.values(state.messages);
      if (values.length === 0) return "";
      if (typeof values[0].ft_app_specific === "string") {
        return values[0].ft_app_specific;
      }
      return null;
    },

    selectMessagesFromEndNode: (state) => {
      return getPathToEndNode(
        state.endNumber,
        state.endAlt,
        state.endPrevAlt,
        Object.values(state.messages),
      );
    },
  },

  extraReducers(builder) {
    builder.addCase(pagesSlice.actions.push, (state, action) => {
      if (
        action.payload.name === "chat" &&
        action.payload.ft_id !== state.ft_id
      ) {
        state = {
          ...initialState,
          ft_id: action.payload.ft_id ?? null,
        };
        return state;
      }
    });

    builder.addCase(createThreadWithMessage.pending, (state) => {
      state.isWaiting = true;
    });
    builder.addCase(createThreadWithMessage.rejected, (state) => {
      state.isStreaming = false;
      state.isWaiting = false;
    });
    builder.addCase(createMessage.pending, (state) => {
      state.isWaiting = true;
    });
    builder.addCase(createMessage.rejected, (state) => {
      state.isStreaming = false;
      state.isWaiting = false;
    });
  },
});

export const {
  receiveThreadMessages,
  setThreadEnd,
  resetThread,
  setThreadFtId,
} = threadMessagesSlice.actions;
export const {
  selectThreadMessages,
  selectIsStreaming,
  selectIsWaiting,
  selectIsWaitingOrStreaming,
  selectThreadId,
  selectThreadMessageTrie,
  selectThreadEnd,
  isThreadEmpty,
  selectAppSpecific,
  selectMessagesFromEndNode,
} = threadMessagesSlice.selectors;
