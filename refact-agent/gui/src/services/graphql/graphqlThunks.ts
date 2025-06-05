import {
  Client,
  createGraphqlClient,
  createSubscription,
} from "./createClient";
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
  MessageCreateMutationVariables,
  MessageCreateMutation,
  MessageCreateDocument,
  MessagesSubscriptionSubscription,
  FThreadMessageInput,
} from "../../../generated/documents";
import { handleThreadListSubscriptionData } from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";
import { AppDispatch, RootState } from "../../app/store";
import {
  receiveThreadMessages,
  setThreadFtId,
} from "../../features/ThreadMessages";
import { appSearchableIdsApi } from "../refact/AppSearchAbleIds";

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

export const createThreadThunk = createAsyncThunk<
  CreateThreadMutation,
  CreateThreadMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("graphql/createThread", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  // before creating get the id from  http://localhost:8001/v1/get-app-searchable-id

  const client = createGraphqlClient(apiKey, thunkAPI.signal);

  const result = await client.mutation<
    CreateThreadMutation,
    CreateThreadMutationVariables
  >(CreateThreadDocument, args);
  if (result.error) {
    return thunkAPI.rejectWithValue({
      message: result.error.message,
    });
  }

  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "Failed to create thread" });
  }
  return thunkAPI.fulfillWithValue(result.data);
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";
  console.log(thunkApi.requestId);
  const sub = createSubscription<
    MessagesSubscriptionSubscription,
    MessagesSubscriptionSubscriptionVariables
  >(apiKey, MessagesSubscriptionDocument, args, thunkApi.signal, (result) => {
    console.log(thunkApi.requestId);
    if (thunkApi.signal.aborted) {
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
  MessageCreateMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; args: MessageCreateMutationVariables };
  }
>("graphql/createMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const client = createGraphqlClient(apiKey, thunkAPI.signal);
  const result = await client.mutation<
    MessageCreateMutation,
    MessageCreateMutationVariables
  >(MessageCreateDocument, args);

  if (result.error) {
    console.log(result);
    thunkAPI.dispatch(setError(result.error.message));
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      args,
    });
  }
  return thunkAPI.fulfillWithValue(result.data);
});

export const createThreadWithMessage = createAsyncThunk<
  unknown,
  Pick<FThreadMessageInput, "ftm_content">,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("graphql/createThreadWithMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const workspace = state.config.currentWorkspaceName ?? "";
  const appId = await thunkAPI.dispatch(
    appSearchableIdsApi.endpoints.getAppSearchableId.initiate(undefined),
  );
  if (appId.error) {
    thunkAPI.rejectWithValue({ message: JSON.stringify(appId.error) });
  }
  // if (!appId.data?.app_searchable_id) {
  //   thunkAPI.rejectWithValue({ message: "No App Id" });
  // }

  const threadQueryArgs = {
    input: {
      ft_fexp_name: "ask",
      ft_title: "", // TODO: generate the title
      located_fgroup_id: workspace,
      owner_shared: false,
      ft_app_searchable: appId.data?.app_searchable_id,
    },
  };
  const threadQuery = await createGraphqlClient(
    apiKey,
    thunkAPI.signal,
  ).mutation<CreateThreadMutation, CreateThreadMutationVariables>(
    CreateThreadDocument,
    threadQueryArgs,
  );

  if (threadQuery.error) {
    return thunkAPI.rejectWithValue({ message: threadQuery.error.message });
  }

  if (threadQuery.data && state.threadMessages.ft_id === null) {
    thunkAPI.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
  }

  const result = await thunkAPI.dispatch(
    createMessage({
      input: {
        ftm_app_specific: appId.data?.app_searchable_id,
        ftm_belongs_to_ft_id: threadQuery.data?.thread_create.ft_id ?? "",
        ftm_alt: 100,
        ftm_num: 1,
        ftm_call_id: "",
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_content: JSON.stringify(args.ftm_content),
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
      },
    }),
  );
  return result;
});

// TODO: stop is ft_error, set this and it'll stop
