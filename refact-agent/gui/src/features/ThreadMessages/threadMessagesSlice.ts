import {
  createSelector,
  createSlice,
  type PayloadAction,
} from "@reduxjs/toolkit";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";
import { makeMessageTrie, getAncestorsForNode } from "./makeMessageTrie";
import type { BaseMessage } from "../../services/refact/types";
import { pagesSlice } from "../Pages/pagesSlice";
import {
  graphqlQueriesAndMutations,
  messagesSub,
} from "../../services/graphql";

import {
  isDiffMessage,
  isToolCall,
  ToolMessage,
  isToolMessage,
} from "../../services/refact";

import { Override, takeWhile } from "../../utils";

// TODO: move this somewhere
export type ToolConfirmationRequest = {
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

type Thread = NonNullable<
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread"]
>;

type Delta = NonNullable<
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["stream_delta"]
>;

type Message = NonNullable<
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread_message"]
>;

export type IntegrationMeta = {
  name?: string;
  path?: string;
  project?: string;
  shouldIntermediatePageShowUp?: boolean;
};

export function isIntegrationMeta(json: unknown): json is IntegrationMeta {
  if (!json || typeof json !== "object") return false;
  if (!("name" in json) || !("path" in json) || !("project" in json)) {
    return false;
  }
  return true;
}

export type MessageWithIntegrationMeta = Override<
  Message,
  {
    ftm_user_preferences: { integration: IntegrationMeta };
  }
>;

export function isMessageWithIntegrationMeta(
  message: unknown,
): message is MessageWithIntegrationMeta {
  if (!message || typeof message !== "object") return false;
  if (!("ftm_user_preferences" in message)) return false;
  if (
    !message.ftm_user_preferences ||
    typeof message.ftm_user_preferences !== "object"
  )
    return false;
  const preferences = message.ftm_user_preferences as Record<string, unknown>;
  if (!("integration" in preferences)) return false;
  return isIntegrationMeta(preferences.integration);
}

export type MessagesInitialState = {
  waitingBranches: number[]; // alt numbers
  streamingBranches: number[]; // alt number
  messages: Record<string, BaseMessage>;
  ft_id: string | null;
  endNumber: number;
  endAlt: number;
  endPrevAlt: number;
  thread: Thread | null;
  loading: boolean;
};

const initialState: MessagesInitialState = {
  waitingBranches: [],
  streamingBranches: [],
  messages: {},
  ft_id: null,
  endNumber: 0,
  endAlt: 0,
  endPrevAlt: 0,
  thread: null,
  loading: false,
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
  (state: MessagesInitialState) => state.messages,
  (messages) => Object.values(messages),
);

export const threadMessagesSlice = createSlice({
  name: "threadMessages",
  initialState,
  reducers: {
    receiveThread: (
      state,
      action: PayloadAction<{
        news_action: string;
        news_payload_id: string;
        news_payload_thread: Thread;
      }>,
    ) => {
      if (state.thread === null && action.payload.news_action === "UPDATE") {
        state.thread = action.payload.news_payload_thread;
      } else if (
        state.thread &&
        action.payload.news_payload_id !== state.thread.ft_id
      ) {
        return state;
      } else {
        state.thread = action.payload.news_payload_thread;
      }

      if (
        action.payload.news_payload_thread.ft_need_assistant &&
        action.payload.news_payload_thread.ft_need_assistant !== -1
      ) {
        state.waitingBranches.push(
          action.payload.news_payload_thread.ft_need_assistant,
        );
      }

      if (
        action.payload.news_payload_thread.ft_need_user &&
        action.payload.news_payload_thread.ft_need_user !== -1
      ) {
        state.waitingBranches = state.waitingBranches.filter(
          (n) => n !== action.payload.news_payload_thread.ft_need_user,
        );
        state.streamingBranches = state.streamingBranches.filter(
          (n) => n !== action.payload.news_payload_thread.ft_need_user,
        );
      }

      // return state;
      // thread updates
    },
    receiveDeltaStream: (
      state,
      action: PayloadAction<{
        news_action: string;
        news_payload_id: string;
        stream_delta: Delta;
      }>,
    ) => {
      if (action.payload.news_action !== "DELTA") return state;
      if (
        !state.thread?.ft_id ||
        !action.payload.news_payload_id.startsWith(state.thread.ft_id)
      ) {
        return state;
      }

      if (
        action.payload.news_payload_id in state.messages &&
        "ftm_content" in action.payload.stream_delta
      ) {
        // TODO: multimodal could break this
        state.messages[action.payload.news_payload_id].ftm_content +=
          action.payload.stream_delta.ftm_content;
        return state;
      }

      const infoFromId = getInfoFromId(action.payload.news_payload_id);
      if (!infoFromId) return state;
      if (!(action.payload.news_payload_id in state.messages)) {
        const msg: BaseMessage = {
          ...infoFromId,
          ftm_role: action.payload.stream_delta.ftm_role,
          // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
          ftm_content: action.payload.stream_delta.ftm_content,
          // TODO: these
          ftm_call_id: "",
          ftm_created_ts: 0,
        };
        state.messages[action.payload.news_payload_id] = msg;
      }

      if (!state.streamingBranches.includes(infoFromId.ftm_alt)) {
        state.streamingBranches.push(infoFromId.ftm_alt);
        state.waitingBranches = state.waitingBranches.filter(
          (n) => n !== infoFromId.ftm_alt,
        );
      }
    },

    removeMessage: (
      state,
      action: PayloadAction<{ news_action: string; news_payload_id: string }>,
    ) => {
      if (action.payload.news_action !== "DELETE") return state;
      const messages = Object.entries(state.messages).reduce<
        typeof state.messages
      >((acc, [key, value]) => {
        if (key === action.payload.news_payload_id) {
          return acc;
        }
        return { ...acc, [key]: value };
      }, {});

      state.messages = messages;
      return state;
    },

    receiveThreadMessages: (
      state,
      action: PayloadAction<{
        news_action: string;
        news_payload_id: string;
        news_payload_thread_message: Message;
      }>, // change this to FThreadMessageOutput
    ) => {
      if (!state.thread) return state;

      if (!action.payload.news_payload_id.startsWith(state.thread.ft_id)) {
        return state;
      }

      // TODO: are there other cases aside from update
      // actions: INITIAL_UPDATES_OVER | UPDATE | DELETE
      if (action.payload.news_action === "UPDATE") {
        state.messages[action.payload.news_payload_id] =
          action.payload.news_payload_thread_message;

        state.waitingBranches = state.waitingBranches.filter(
          (n) => n !== action.payload.news_payload_thread_message.ftm_alt,
        );
      }

      if (action.payload.news_action === "INSERT") {
        state.messages[action.payload.news_payload_id] =
          action.payload.news_payload_thread_message;

        state.waitingBranches = state.waitingBranches.filter(
          (n) => n !== action.payload.news_payload_thread_message.ftm_alt,
        );
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
    setThreadFtId: (
      state,
      action: PayloadAction<MessagesInitialState["ft_id"]>,
    ) => {
      state.ft_id = action.payload;
    },

    setLoading: (
      state,
      action: PayloadAction<{ ft_id: string; loading: boolean }>,
    ) => {
      if (action.payload.ft_id !== state.ft_id) return;
      state.loading = action.payload.loading;
    },
  },
  selectors: {
    selectThreadMessages: (state) => Object.values(state.messages),
    selectThreadMeta: (state) => state.thread,
    selectThreadId: (state) => state.ft_id,
    selectIsWaiting: (state) => {
      const maybeBranch = state.waitingBranches.find(
        (branch) => branch === state.endAlt,
      );
      return !!maybeBranch;
    },
    selectIsStreaming: (state) => {
      if (state.streamingBranches.length === 0) return false;
      const maybeBranch = state.streamingBranches.find(
        (branch) => branch === state.endAlt,
      );
      return !!maybeBranch;
    },
    selectLoading: (state) => state.loading,
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
    selectMessagesFromEndNode: (state) => {
      const { endNumber, endAlt, endPrevAlt, messages } = state;
      return getAncestorsForNode(
        endNumber,
        endAlt,
        endPrevAlt,
        Object.values(messages),
      );
    },

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

    // TODO: fix this once the lsp is working again :/
    selectToolConfirmationRequests: (state) => {
      if (!state.thread) return [];
      // if (
      //   Array.isArray(state.thread.ft_confirmation_response) &&
      //   state.thread.ft_confirmation_response.includes("*")
      // ) {
      //   return [];
      // }
      const messages = Object.values(state.messages);
      if (messages.length === 0) return [];
      if (!state.thread.ft_confirmation_request) return [];
      if (!Array.isArray(state.thread.ft_confirmation_request)) return [];
      // const responses = Array.isArray(state.thread.ft_confirmation_response)
      //   ? state.thread.ft_confirmation_response
      //   : [];
      const toolRequests = state.thread.ft_confirmation_request.filter(
        isToolConfirmationRequest,
      );

      const messageIds = messages.map((message) => message.ftm_call_id);
      // const unresolved = toolRequests.filter(
      //   (req) =>
      //     !responses.includes(req.tool_call_id) &&
      //     !messageIds.includes(req.tool_call_id),
      // );
      const unresolved = toolRequests.filter(
        (req) => !messageIds.includes(req.tool_call_id),
      );

      return unresolved;
    },

    selectToolConfirmationResponses: (state) => {
      if (!state.thread) return [];
      return [];
      // if (!Array.isArray(state.thread.ft_confirmation_response)) {
      //   return [];
      // }

      // return state.thread.ft_confirmation_response.filter(
      //   (s) => typeof s === "string",
      // );
    },
    // TODO: figure this out
    selectPatchIsAutomatic: (state) => {
      if (!state.thread) return false;
      return false;
      // return (
      //   Array.isArray(state.thread.ft_confirmation_response) &&
      //   state.thread.ft_confirmation_response.includes("*")
      // );
    },
    selectMessageByToolCallId: createSelector(
      [selectMessagesValues, (_messages, id: string) => id],
      (messages, id) => {
        return messages.find((message) => {
          if (!Array.isArray(message.ftm_tool_calls)) return false;
          return message.ftm_tool_calls
            .filter(isToolCall)
            .some((toolCall) => toolCall.id === id);
        });
      },
    ),

    selectLastMessageForAlt: createSelector(
      [selectMessagesValues, (_messages, alt: number) => alt],
      (messages, alt) => {
        const messagesForAlt = messages.filter(
          (message) => message.ftm_alt === alt,
        );
        if (messagesForAlt.length === 0) return null;
        const last = messagesForAlt.sort((a, b) => b.ftm_num - a.ftm_num)[0];
        return last;
      },
    ),

    selectManyDiffMessageByIds: createSelector(
      [selectMessagesValues, (_messages, ids: string[]) => ids],
      (messages, ids) => {
        const diffs = messages.filter(isDiffMessage);
        return diffs.filter((message) => ids.includes(message.ftm_call_id));
      },
    ),

    selectIntegrationMeta: createSelector(selectMessagesValues, (messages) => {
      const maybeIntegrationMeta = messages.find(isMessageWithIntegrationMeta);
      if (!maybeIntegrationMeta) return null;
      // TODO: any types are causing issues here
      const message = maybeIntegrationMeta;
      return message.ftm_user_preferences.integration;
    }),

    selectMessageIsLastOfType: (state, message: BaseMessage) => {
      const { endNumber, endAlt, endPrevAlt, messages } = state;
      const currentBranch = getAncestorsForNode(
        endNumber,
        endAlt,
        endPrevAlt,
        Object.values(messages),
      );
      const hasMessageInBranch = currentBranch.some((msg) => {
        return (
          msg.ftm_num === message.ftm_num &&
          msg.ftm_alt === message.ftm_alt &&
          msg.ftm_prev_alt === message.ftm_prev_alt
        );
      });

      if (!hasMessageInBranch) return false;
      const tail = takeWhile(currentBranch, (msg) => {
        return msg.ftm_num > message.ftm_num;
      });

      if (tail.length === 0) return true;
      const hasMore = tail.some((msg) => msg.ftm_role === message.ftm_role);
      return !hasMore;
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

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.pauseThread.matchFulfilled,
      (state, action) => {
        if (action.payload.thread_patch.ft_id !== state.ft_id) return state;
        state.waitingBranches = [];
        state.streamingBranches = [];
      },
    );

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.createThreadWithSingleMessage
        .matchRejected,
      (state) => {
        state.waitingBranches = state.waitingBranches.filter((n) => n !== 100);
      },
    );

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.createThreadWithSingleMessage
        .matchPending,
      (state) => {
        state.waitingBranches.push(100);
      },
    );

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.sendMessages.matchPending,
      (state, action) => {
        const { input } = action.meta.arg.originalArgs;
        if (input.ftm_belongs_to_ft_id !== state.ft_id) return state;
        state.waitingBranches.push(input.messages[0].ftm_alt);
      },
    );

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.sendMessages.matchRejected,
      (state, action) => {
        const { input } = action.meta.arg.originalArgs;
        if (input.ftm_belongs_to_ft_id !== state.ft_id) return state;
        state.waitingBranches = state.waitingBranches.filter(
          (n) => n !== input.messages[0].ftm_alt,
        );
      },
    );

    builder.addMatcher(messagesSub.pending.match, (state, action) => {
      if (action.meta.arg.ft_id === state.ft_id) {
        state.loading = true;
      }
    });
  },
});

export const {
  receiveDeltaStream,
  receiveThread,
  receiveThreadMessages,
  removeMessage,
  setThreadEnd,
  resetThread,
  setThreadFtId,
  setLoading,
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
  selectThreadMeta,
  selectMessageByToolCallId,
  selectLastMessageForAlt,
  selectPatchIsAutomatic,
  selectToolConfirmationResponses,
  selectManyDiffMessageByIds,
  selectIntegrationMeta,
  selectMessageIsLastOfType,
  selectLoading,
} = threadMessagesSlice.selectors;
