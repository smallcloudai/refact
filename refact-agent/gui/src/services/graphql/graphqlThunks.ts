import { createGraphqlClient } from "./createClient";
import { createAppAsyncThunk } from "./createAppAsyncThunk";

import {
  ThreadsPageSubsDocument,
  ThreadsPageSubsSubscription,
  ThreadsPageSubsSubscriptionVariables,
} from "../../../generated/documents";

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
    console.log(result);
    if (result.data) {
      // ...update history slice
    } else if (result.error) {
      // ... handle error
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
