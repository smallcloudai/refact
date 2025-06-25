import {
  createSelector,
  createSlice,
  type PayloadAction,
} from "@reduxjs/toolkit";
import {
  FThreadMessageOutput,
  FThreadMessageSubs,
  FThreadOutput,
  MessagesSubscriptionSubscription,
} from "../../../generated/documents";
import {
  FTMMessage,
  makeMessageTrie,
  getAncestorsForNode,
} from "./makeMessageTrie";
import { pagesSlice } from "../Pages/pagesSlice";
import {
  createMessage,
  createThreadWithMessage,
  pauseThreadThunk,
} from "../../services/graphql/graphqlThunks";
import { isToolMessage } from "../../events";
import {
  isAssistantMessage,
  isCDInstructionMessage,
  isToolCall,
  ToolMessage,
} from "../../services/refact";

// TODO: move this somewhere
type ToolConfirmationRequest = {
  rule: string; // "default"
  command: string;
  ftm_num: number;
  tool_call_id: string;
};

function isToolConfirmationRequest(
  toolReq: unknown,
): toolReq is ToolConfirmationRequest {
  if (!toolReq) return false;
  if (typeof toolReq !== "object") return false;
  if (!("rule" in toolReq)) return false;
  if (typeof toolReq.rule !== "string") return false;
  if (!("command" in toolReq)) return false;
  if (typeof toolReq.command !== "string") return false;
  if (!("ftm_num" in toolReq)) return false;
  if (typeof toolReq.ftm_num !== "number") return false;
  if (!("tool_call_id" in toolReq)) return false;
  if (typeof toolReq.tool_call_id !== "string") return false;
  return true;
}

type InitialState = {
  waitingBranches: number[]; // alt numbers
  streamingBranches: number[]; // alt number
  messages: Record<string, FTMMessage>;
  ft_id: string | null;
  endNumber: number;
  endAlt: number;
  endPrevAlt: number;
  thread: FThreadOutput | null;
};

const initialState: InitialState = {
  waitingBranches: [],
  streamingBranches: [],
  messages: {},
  ft_id: null,
  endNumber: 0,
  endAlt: 0,
  endPrevAlt: 0,
  thread: null,
};

const ID_REGEXP = /^(.*):(\d+):(\d+):(\d+)$/;

function getInfoFromId(id: string) {
  const result = id.match(ID_REGEXP);
  if (result === null) return null;
  const [_, ftm_belongs_to_ft_id, ftm_alt, ftm_num, ftm_prev_alt] = result;
  return {
    ftm_belongs_to_ft_id,
    ftm_alt: +ftm_alt,
    ftm_num: +ftm_num,
    ftm_prev_alt: +ftm_prev_alt,
  };
}

// https://github.com/reduxjs/redux-toolkit/discussions/4553 see this for creating memoized selectors

const selectMessagesValues = createSelector(
  (state: InitialState) => state.messages,
  (messages) => Object.values(messages),
);

export const threadMessagesSlice = createSlice({
  name: "threadMessages",
  initialState,
  reducers: {
    receiveThreadMessages: (
      state,
      action: PayloadAction<MessagesSubscriptionSubscription>,
    ) => {
      console.log(
        "receiveMessages",
        action.payload.comprehensive_thread_subs.news_action,
        action.payload,
      );

      if (
        state.thread === null &&
        action.payload.comprehensive_thread_subs.news_action === "UPDATE" &&
        action.payload.comprehensive_thread_subs.news_payload_thread
      ) {
        // TODO: some type error
        state.thread = action.payload.comprehensive_thread_subs
          .news_payload_thread as FThreadOutput;
      } else if (
        state.thread &&
        action.payload.comprehensive_thread_subs.news_payload_thread &&
        !action.payload.comprehensive_thread_subs.news_payload_id.startsWith(
          state.thread.ft_id,
        )
      ) {
        return state;
      }

      if (
        action.payload.comprehensive_thread_subs.news_payload_thread
          ?.ft_need_assistant &&
        action.payload.comprehensive_thread_subs.news_payload_thread
          .ft_need_assistant !== -1
      ) {
        state.waitingBranches.push(
          action.payload.comprehensive_thread_subs.news_payload_thread
            .ft_need_assistant,
        );
      }

      if (
        action.payload.comprehensive_thread_subs.news_payload_thread
          ?.ft_need_user &&
        action.payload.comprehensive_thread_subs.news_payload_thread
          .ft_need_user !== -1
      ) {
        state.waitingBranches = state.waitingBranches.filter(
          (n) =>
            n !==
            action.payload.comprehensive_thread_subs.news_payload_thread
              ?.ft_need_user,
        );
        state.streamingBranches = state.streamingBranches.filter(
          (n) =>
            n !==
            action.payload.comprehensive_thread_subs.news_payload_thread
              ?.ft_need_user,
        );
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

      const infoFromId = getInfoFromId(
        action.payload.comprehensive_thread_subs.news_payload_id,
      );

      if (
        action.payload.comprehensive_thread_subs.news_action === "DELTA" &&
        action.payload.comprehensive_thread_subs.stream_delta
      ) {
        if (
          action.payload.comprehensive_thread_subs.news_payload_id in
            state.messages &&
          "ftm_content" in action.payload.comprehensive_thread_subs.stream_delta
        ) {
          // TODO: multimodal could break this
          state.messages[
            action.payload.comprehensive_thread_subs.news_payload_id
          ].ftm_content +=
            action.payload.comprehensive_thread_subs.stream_delta.ftm_content;
        } else if (
          infoFromId &&
          !(
            action.payload.comprehensive_thread_subs.news_payload_id in
            state.messages
          )
        ) {
          const msg: FTMMessage = {
            ...infoFromId,
            ftm_role:
              action.payload.comprehensive_thread_subs.stream_delta.ftm_role,
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            ftm_content:
              action.payload.comprehensive_thread_subs.stream_delta.ftm_content,

            // TODO: these
            ftm_call_id: "",
            ftm_created_ts: 0,
          };
          state.messages[
            action.payload.comprehensive_thread_subs.news_payload_id
          ] = msg;
          if (!state.streamingBranches.includes(infoFromId.ftm_alt)) {
            state.streamingBranches.push(infoFromId.ftm_alt);
            state.waitingBranches = state.waitingBranches.filter(
              (n) => n !== infoFromId.ftm_alt,
            );
          }
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

    // TODO: check where this is used
    setThreadFtId: (state, action: PayloadAction<InitialState["ft_id"]>) => {
      state.ft_id = action.payload;
    },
  },
  selectors: {
    selectThreadMessages: (state) => Object.values(state.messages),
    selectThreadId: (state) => state.ft_id,
    selectIsWaiting: (state) => {
      const maybeBranch = state.waitingBranches.find(
        (branch) => branch === state.endAlt,
      );
      return !!maybeBranch;
    },
    selectIsStreaming: (state) => {
      const maybeBranch = state.streamingBranches.find(
        (branch) => branch === state.endAlt,
      );
      return !!maybeBranch;
    },
    selectThreadMessageTrie: createSelector(selectMessagesValues, (messages) =>
      makeMessageTrie(messages),
    ),
    selectThreadEnd: (state) => {
      const { endNumber, endAlt, endPrevAlt } = state;
      return { endNumber, endAlt, endPrevAlt };
    },
    isThreadEmpty: createSelector(
      selectMessagesValues,
      (messages) => messages.length === 0,
    ),
    selectAppSpecific: createSelector(selectMessagesValues, (messages) => {
      if (messages.length === 0) return "";
      if (typeof messages[0].ft_app_specific === "string") {
        return messages[0].ft_app_specific;
      }
      return null;
    }),

    // TODO: refactor this
    selectMessagesFromEndNode: createSelector(
      (state: InitialState) => {
        const { endNumber, endAlt, endPrevAlt, messages } = state;
        return { endNumber, endAlt, endPrevAlt, messages };
      },
      ({ endAlt, endNumber, endPrevAlt, messages }) => {
        return getAncestorsForNode(
          endNumber,
          endAlt,
          endPrevAlt,
          Object.values(messages),
        );
      },
    ),

    selectBranchLength: (state) => state.endNumber,
    selectTotalMessagesInThread: createSelector(
      selectMessagesValues,
      (messages) => messages.length,
    ),
    selectThreadMessagesIsEmpty: createSelector(
      selectMessagesValues,
      (messages) => messages.length === 0,
    ),

    selectThreadMessageTopAltNumber: createSelector(
      selectMessagesValues,
      (messages) => {
        if (messages.length === 0) return null;
        const alts = messages.map((message) => message.ftm_alt);
        return Math.max(...alts);
      },
    ),

    selectIsThreadRunning: (state) => {
      if (state.waitingBranches.length > 0) return true;
      if (state.streamingBranches.length > 0) return true;
      return false;
    },
    /**
     * 
     * export const selectManyToolResultsByIds = (ids: string[]) =>
       createSelector(toolMessagesSelector, (messages) => {
         return messages
           .filter((message) => ids.includes(message.ftm_content.tool_call_id))
           .map((toolMessage) => toolMessage.ftm_content);
       });
     */

    selectManyToolMessagesByIds: createSelector(
      [selectMessagesValues, (_messages, ids: string[]) => ids],
      (messages, ids) => {
        const toolMessages = messages.reduce<ToolMessage[]>((acc, message) => {
          if (!isToolMessage(message)) return acc;
          if (!ids.includes(message.ftm_call_id)) return acc;
          return [...acc, message];
        }, []);

        return toolMessages;
      },
    ),

    selectToolMessageById: createSelector(
      [selectMessagesValues, (_messages, id?: string) => id],
      (messages, id) => {
        return messages.find((message) => {
          if (!isToolMessage(message)) return false;
          return message.ftm_call_id === id;
        });
      },
    ),

    selectToolConfirmationRequests: (state) => {
      if (!state.thread) return [];
      const messages = Object.values(state.messages);
      if (messages.length === 0) return [];
      if (!state.thread.ft_confirmation_request) return [];
      if (!Array.isArray(state.thread.ft_confirmation_request)) return [];
      const toolRequests = state.thread.ft_confirmation_request.filter(
        isToolConfirmationRequest,
      );

      return toolRequests;
      // TBD: do request accumulate after they are called?
      // const maybeArray = Array.isArray(state.thread.ft_confirmation_response)
      //   ? state.thread.ft_confirmation_response
      //   : [];
      // const confirmed: string[] = maybeArray.filter(
      //   (res) => typeof res === "string",
      // );
      // note rejected messages are tool messages sent by the user
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
      state.waitingBranches.push(100);
    });
    builder.addCase(createThreadWithMessage.rejected, (state) => {
      state.waitingBranches = state.waitingBranches.filter((n) => n !== 100);
    });

    builder.addCase(createMessage.pending, (state, action) => {
      const { input } = action.meta.arg;
      if (input.ftm_belongs_to_ft_id !== state.ft_id) return state;
      state.waitingBranches.push(input.messages[0].ftm_alt);
    });
    builder.addCase(createMessage.rejected, (state, action) => {
      const { input } = action.meta.arg;
      if (input.ftm_belongs_to_ft_id !== state.ft_id) return state;
      state.waitingBranches = state.waitingBranches.filter(
        (n) => n !== input.messages[0].ftm_alt,
      );
    });

    builder.addCase(pauseThreadThunk.fulfilled, (state, action) => {
      if (action.payload.thread_patch.ft_id !== state.ft_id) return state;
      state.waitingBranches = [];
      state.streamingBranches = [];
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
  selectThreadId,
  selectThreadMessageTrie,
  selectThreadEnd,
  isThreadEmpty,
  selectAppSpecific,
  selectMessagesFromEndNode,
  selectThreadMessagesIsEmpty,
  selectTotalMessagesInThread,
  selectBranchLength,
  selectThreadMessageTopAltNumber,
  selectIsThreadRunning,
  selectManyToolMessagesByIds,
  selectToolMessageById,
  selectToolConfirmationRequests,
} = threadMessagesSlice.selectors;
