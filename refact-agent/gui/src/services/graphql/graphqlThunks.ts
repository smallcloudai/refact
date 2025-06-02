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
} from "../../../generated/documents";
import { handleThreadListSubscriptionData } from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";
import { AppDispatch, RootState } from "../../app/store";
// import { CombinedError } from "urql";

const THREE_MINUTES = 3 * 60 * 1000;

export const threadsPageSub = createAppAsyncThunk<
  unknown,
  ThreadsPageSubsSubscriptionVariables
>("graphql/threadsPageSub", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  // TODO: make this reusable in other subscriptions
  const client = createGraphqlClient(apiKey, thunkAPI.signal);
  const query = client.subscription<
    ThreadsPageSubsSubscription,
    ThreadsPageSubsSubscriptionVariables
  >(ThreadsPageSubsDocument, args);

  const handleResult: Parameters<typeof query.subscribe>[0] = (result) => {
    if (result.data) {
      thunkAPI.dispatch(handleThreadListSubscriptionData(result.data));
    } else if (result.error) {
      thunkAPI.dispatch(setError(result.error.message));
    }
  };

  let subscription = query.subscribe(handleResult);

  let paused = false;
  let timeout: number | null | NodeJS.Timeout = null;

  function maybeClearTimeout() {
    if (timeout !== null) {
      clearTimeout(timeout);
      timeout = null;
    }
  }

  const handleVisibilityChange = () => {
    if (document.hidden && !paused) {
      maybeClearTimeout();
      timeout = setTimeout(() => {
        paused = true;
        subscription.unsubscribe();
      }, THREE_MINUTES);
    } else if (!document.hidden && paused) {
      paused = false;
      maybeClearTimeout();
      subscription = query.subscribe(handleResult);
    }
  };
  document.addEventListener("visibilitychange", handleVisibilityChange);
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
  unknown,
  CreateThreadMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("graphql/createThread", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

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
  return thunkAPI.fulfillWithValue(result.data ?? args);
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";
  createSubscription(
    apiKey,
    MessagesSubscriptionDocument,
    args,
    thunkApi.signal,
    (result) => {
      console.log(result.data);
    },
  );
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
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      args,
    });
  }
  return thunkAPI.fulfillWithValue(result.data);
});
