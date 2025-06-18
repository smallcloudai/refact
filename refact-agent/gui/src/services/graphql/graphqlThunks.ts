import { createGraphqlClient, createSubscription } from "./createClient";
import { createAppAsyncThunk } from "./createAppAsyncThunk";
import { createAsyncThunk } from "@reduxjs/toolkit";

import {
  ThreadsPageSubsDocument,
  ThreadsPageSubsSubscription,
  ThreadsPageSubsSubscriptionVariables,
  DeleteThreadDocument,
  DeleteThreadMutationVariables,
  DeleteThreadMutation,
  CreateThreadMutation,
  CreateThreadDocument,
  CreateThreadMutationVariables,
  MessagesSubscriptionSubscriptionVariables,
  MessagesSubscriptionDocument,
  MessageCreateMultipleDocument,
  MessageCreateMultipleMutation,
  MessageCreateMultipleMutationVariables,
  MessagesSubscriptionSubscription,
  FThreadMessageInput,
  FThreadInput,
  ThreadPatchDocument,
  ThreadPatchMutation,
  ThreadPatchMutationVariables,
} from "../../../generated/documents";
import { handleThreadListSubscriptionData } from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";
import { AppDispatch, RootState } from "../../app/store";
import {
  receiveThreadMessages,
  setThreadFtId,
} from "../../features/ThreadMessages";

export const threadsPageSub = createAppAsyncThunk<
  unknown,
  ThreadsPageSubsSubscriptionVariables
>("graphql/threadsPageSub", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  createSubscription<
    ThreadsPageSubsSubscription,
    ThreadsPageSubsSubscriptionVariables
  >(apiKey, ThreadsPageSubsDocument, args, thunkAPI.signal, (result) => {
    if (result.data) {
      thunkAPI.dispatch(handleThreadListSubscriptionData(result.data));
    } else if (result.error) {
      thunkAPI.dispatch(setError(result.error.message));
    }
  });
});

export const deleteThreadThunk = createAsyncThunk<
  { id: string },
  DeleteThreadMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; id: string };
  }
>("graphql/deleteThread", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const client = createGraphqlClient(apiKey, thunkAPI.signal);
  const result = await client.mutation<
    DeleteThreadMutation,
    DeleteThreadMutationVariables
  >(DeleteThreadDocument, args);
  if (result.error) {
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      id: args.id,
    });
  }
  return thunkAPI.fulfillWithValue({ id: args.id });
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";

  const sub = createSubscription<
    MessagesSubscriptionSubscription,
    MessagesSubscriptionSubscriptionVariables
  >(apiKey, MessagesSubscriptionDocument, args, thunkApi.signal, (result) => {
    if (thunkApi.signal.aborted) {
      // eslint-disable-next-line no-console
      console.log("handleResult called after thunk signal is aborted");

      return thunkApi.fulfillWithValue({});
    }
    if (result.error) {
      // TBD: do we hang up on errors?
      thunkApi.dispatch(setError(result.error.message));
    }
    if (result.data) {
      thunkApi.dispatch(receiveThreadMessages(result.data));
    }
  });

  // TODO: duplicated call to unsubscribe
  thunkApi.signal.addEventListener("abort", () => {
    sub.unsubscribe();
    thunkApi.fulfillWithValue({});
  });

  // return thunkApi.fulfillWithValue({});
});

export const createMessage = createAppAsyncThunk<
  unknown,
  MessageCreateMultipleMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: {
      message: string;
      args: MessageCreateMultipleMutationVariables;
    };
  }
>("graphql/createMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const client = createGraphqlClient(apiKey, thunkAPI.signal);
  const result = await client.mutation<
    MessageCreateMultipleMutation,
    MessageCreateMultipleMutationVariables
  >(MessageCreateMultipleDocument, args);

  if (result.error) {
    thunkAPI.dispatch(setError(result.error.message));
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      args,
    });
  }
  // TODO: add the message to the message list
  return thunkAPI.fulfillWithValue(result.data);
});

export const createThreadWithMessage = createAsyncThunk<
  MessageCreateMultipleMutation,
  { content: string },
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("graphql/createThreadWithMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const port = state.config.lspPort;
  // TODO: where is current workspace set?
  const workspace = state.config.currentWorkspaceName ?? "solar_root";

  const appIdUrl = `http://127.0.0.1:${port}/v1/get-app-searchable-id`;

  const appIdQuery = await fetch(appIdUrl, {
    credentials: "same-origin",
    redirect: "follow",
    headers: { Authorization: `Bearer ${apiKey}` },
  })
    .then((res) => res.json())
    .then((json) => {
      if (!isGetAppSearchableResponse(json)) {
        const message = `failed parse get_app_searchable_id response: ${JSON.stringify(
          json,
        )}`;
        return {
          data: null,
          error: message,
        };
      }
      return {
        data: json,
        error: null,
      };
    })
    .catch((error: Error) => ({ error: error, data: null }));

  if (appIdQuery.error) {
    thunkAPI.rejectWithValue({ message: JSON.stringify(appIdQuery.error) });
  }

  const client = createGraphqlClient(apiKey, thunkAPI.signal);

  const threadQueryArgs: FThreadInput = {
    ft_fexp_id: "id:ask:1.0", // TODO: user selected
    ft_title: "", // TODO: generate the title
    located_fgroup_id: workspace,
    owner_shared: false,
    ft_app_searchable: appIdQuery.data?.app_searchable_id,
  };
  const threadQuery = await client.mutation<
    CreateThreadMutation,
    CreateThreadMutationVariables
  >(CreateThreadDocument, { input: threadQueryArgs });

  if (threadQuery.error) {
    return thunkAPI.rejectWithValue({ message: threadQuery.error.message });
  }

  if (!threadQuery.data) {
    return thunkAPI.rejectWithValue({
      message: "couldn't create flexus thread id",
    });
  }

  if (state.threadMessages.ft_id === null) {
    thunkAPI.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
  }

  // Note: ftm_num, ftm_alt, and ftm_prev_alt are also hard coded for tracking waiting state
  const createMessageArgs: FThreadMessageInput = {
    ftm_app_specific: JSON.stringify(appIdQuery.data?.app_searchable_id ?? ""),
    ftm_belongs_to_ft_id: threadQuery.data.thread_create.ft_id,
    ftm_alt: 100,
    ftm_num: 1,
    ftm_call_id: "",
    ftm_prev_alt: 100,
    ftm_role: "user",
    ftm_content: JSON.stringify(args.content),
    ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
    ftm_tool_calls: "null", // optional
    ftm_usage: "null", // optional
  };

  const result = await client.mutation<
    MessageCreateMultipleMutation,
    MessageCreateMultipleMutationVariables
  >(MessageCreateMultipleDocument, {
    input: {
      ftm_belongs_to_ft_id: threadQuery.data.thread_create.ft_id,
      messages: [createMessageArgs],
    },
  });

  if (result.error) {
    return thunkAPI.rejectWithValue({ message: result.error.message });
  }
  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "failed to create message" });
  }
  return thunkAPI.fulfillWithValue(result.data);
});

// TODO: stop is ft_error, set this and it'll stop

type GetAppSearchableIdResponse = {
  app_searchable_id: string;
};

function isGetAppSearchableResponse(
  response: unknown,
): response is GetAppSearchableIdResponse {
  if (!response) return false;
  if (typeof response !== "object") return false;
  if (!("app_searchable_id" in response)) return false;
  return typeof response.app_searchable_id === "string";
}

export const pauseThreadThunk = createAppAsyncThunk<
  ThreadPatchMutation,
  { id: string },
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("thread/pause", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const client = createGraphqlClient(apiKey, thunkAPI.signal);

  const result = await client.mutation<
    ThreadPatchMutation,
    ThreadPatchMutationVariables
  >(ThreadPatchDocument, { id: args.id, message: JSON.stringify("pause") });

  if (result.error) {
    return thunkAPI.rejectWithValue(result.error);
  }

  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "failed to stop thread" });
  }
  return thunkAPI.fulfillWithValue(result.data);
});
