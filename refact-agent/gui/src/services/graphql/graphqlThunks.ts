import { createGraphqlClient } from "./createClient";
import { createAppAsyncThunk } from "./createAppAsyncThunk";

import {
  ThreadsPageSubsDocument,
  ThreadsPageSubsSubscription,
  ThreadsPageSubsSubscriptionVariables,
} from "../../../generated/documents";
import {
  handleThreadListSubscriptionData,
  setThreadListLoading,
} from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";

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

  thunkAPI.dispatch(setThreadListLoading(true));
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
      thunkAPI.dispatch(setThreadListLoading(true));
      subscription = query.subscribe(handleResult);
    }
  };
  document.addEventListener("visibilitychange", handleVisibilityChange);
});
