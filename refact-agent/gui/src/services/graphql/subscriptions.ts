import { createSubscription } from "./createClient";
import { createAsyncThunk } from "@reduxjs/toolkit";

import {
  ThreadsPageSubsDocument,
  ThreadsPageSubsSubscription,
  ThreadsPageSubsSubscriptionVariables,
  MessagesSubscriptionSubscriptionVariables,
  MessagesSubscriptionDocument,
  MessagesSubscriptionSubscription,
  WorkspaceTreeSubscription,
  WorkspaceTreeSubscriptionVariables,
  WorkspaceTreeDocument,
} from "../../../generated/documents";
import { handleThreadListSubscriptionData } from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";
import { AppDispatch, RootState } from "../../app/store";
import {
  receiveDeltaStream,
  receiveThread,
  receiveThreadMessages,
  removeMessage,
} from "../../features/ThreadMessages";

import { receiveWorkspace, receiveWorkspaceError } from "../../features/Groups";
import { v4 as uuidv4 } from "uuid";

import { connected, connecting, closed } from "../../features/ConnectionStatus";

export const threadsPageSub = createAsyncThunk<
  unknown,
  ThreadsPageSubsSubscriptionVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
  }
>("graphql/threadsPageSub", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const address = state.config.addressURL ?? "https://app.refact.ai";

  const connectionId = uuidv4();

  createSubscription<
    ThreadsPageSubsSubscription,
    ThreadsPageSubsSubscriptionVariables
  >(
    address,
    apiKey,
    ThreadsPageSubsDocument,
    args,
    thunkAPI.signal,
    (result) => {
      if (result.data) {
        thunkAPI.dispatch(handleThreadListSubscriptionData(result.data));
      } else if (result.error) {
        thunkAPI.dispatch(setError(result.error.message));
      }
    },

    {
      connecting: () => {
        thunkAPI.dispatch(
          connecting({ id: connectionId, name: "ThreadsPageSub" }),
        );
      },

      connected: () => {
        thunkAPI.dispatch(
          connected({ id: connectionId, name: "ThreadsPageSub" }),
        );
      },

      closed: () => {
        thunkAPI.dispatch(closed({ id: connectionId }));
      },
    },
  );
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";
  const address = state.config.addressURL ?? "https://app.refact.ai";

  const connectionId = uuidv4();

  const sub = createSubscription<
    MessagesSubscriptionSubscription,
    MessagesSubscriptionSubscriptionVariables
  >(
    address,
    apiKey,
    MessagesSubscriptionDocument,
    args,
    thunkApi.signal,
    (result) => {
      if (thunkApi.signal.aborted) {
        // eslint-disable-next-line no-console
        console.log("handleResult called after thunk signal is aborted");

        return thunkApi.fulfillWithValue({});
      }
      if (result.error) {
        // TBD: do we hang up on errors?
        thunkApi.dispatch(setError(result.error.message));
      }

      if (result.data?.comprehensive_thread_subs.news_payload_thread) {
        thunkApi.dispatch(
          receiveThread({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            news_payload_thread:
              result.data.comprehensive_thread_subs.news_payload_thread,
          }),
        );
      }
      if (result.data?.comprehensive_thread_subs.stream_delta) {
        thunkApi.dispatch(
          receiveDeltaStream({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            stream_delta: result.data.comprehensive_thread_subs.stream_delta,
          }),
        );
      }

      if (result.data?.comprehensive_thread_subs.news_action === "DELETE") {
        thunkApi.dispatch(
          removeMessage({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
          }),
        );
      }

      if (result.data?.comprehensive_thread_subs.news_payload_thread_message) {
        thunkApi.dispatch(
          receiveThreadMessages({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            news_payload_thread_message:
              result.data.comprehensive_thread_subs.news_payload_thread_message,
          }),
        );
      }
    },
    {
      connecting: () => {
        thunkApi.dispatch(
          connecting({ id: connectionId, name: "MessagesSub" }),
        );
      },

      connected: () => {
        thunkApi.dispatch(connected({ id: connectionId, name: "MessagesSub" }));
      },

      closed: () => {
        thunkApi.dispatch(closed({ id: connectionId }));
      },
    },
  );

  // TODO: duplicated call to unsubscribe
  thunkApi.signal.addEventListener("abort", () => {
    sub.unsubscribe();
    thunkApi.fulfillWithValue({});
  });

  // return thunkApi.fulfillWithValue({});
});

export const workspaceTreeSubscriptionThunk = createAsyncThunk<
  unknown,
  WorkspaceTreeSubscriptionVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: {
      message: string;
      args: WorkspaceTreeSubscriptionVariables;
    };
  }
>("flexus/treeSubscription", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;
  const connectionId = uuidv4();
  createSubscription<
    WorkspaceTreeSubscription,
    WorkspaceTreeSubscriptionVariables
  >(
    addressUrl,
    apiKey,
    WorkspaceTreeDocument,
    args,
    thunkAPI.signal,
    (result) => {
      if (result.error) {
        thunkAPI.dispatch(receiveWorkspaceError(result.error.message));
      }
      if (result.data) {
        thunkAPI.dispatch(receiveWorkspace(result.data.tree_subscription));
      }
    },
    {
      connecting: () => {
        thunkAPI.dispatch(
          connecting({ id: connectionId, name: "WorkspaceTreeSub" }),
        );
      },

      connected: () => {
        thunkAPI.dispatch(
          connected({ id: connectionId, name: "WorkSpaceTreeSub" }),
        );
      },

      closed: () => {
        thunkAPI.dispatch(closed({ id: connectionId }));
      },
    },
  );
});
