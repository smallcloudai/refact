import {
  createClient,
  // debugExchange,
  // cacheExchange,
  fetchExchange,
  subscriptionExchange,
  type AnyVariables,
  type DocumentInput,
  type OperationContext,
  type OperationResult,
} from "@urql/core";
import { createClient as createWSClient } from "graphql-ws";
import { WebSocket } from "ws";

const THREE_MINUTES = 3 * 60 * 1000;

export const createGraphqlClient = (
  addressUrl: string,
  apiKey: string,
  signal: AbortSignal,
) => {
  const addr =
    !addressUrl || addressUrl === "Refact"
      ? `https://app.refact.ai`
      : addressUrl;
  const httpUrl = new URL(addr);
  httpUrl.pathname = "/v1/graphql";

  const wsUrl = new URL(addr);
  wsUrl.pathname = "/v1/graphql";
  wsUrl.protocol = addr.startsWith("http://") ? "ws" : "wss";

  const wsClient = createWSClient({
    url: wsUrl.toString(),
    connectionParams: { apikey: apiKey },
    webSocketImpl: WebSocket,
    retryAttempts: 5,
  });

  const urqlClient = createClient({
    url: httpUrl.toString(),
    exchanges: [
      // TODO: only enable this during development
      // debugExchange,
      // cacheExchange,
      subscriptionExchange({
        forwardSubscription: (operation) => ({
          subscribe: (sink) => {
            const payload = { ...operation, query: operation.query ?? "" };
            const dispose = wsClient.subscribe(payload, sink);
            return { unsubscribe: dispose };
          },
        }),
      }),
      fetchExchange,
    ],
    fetchOptions: () => ({
      signal: signal,
      headers: {
        Authorization: `Bearer ${apiKey}`,
      },
    }),
  });

  signal.addEventListener("abort", () => {
    // console.log("aborting wsClient");
    wsClient.terminate();
    void wsClient.dispose();
  });

  return urqlClient;
};

export function createSubscription<
  T = unknown,
  Variables extends AnyVariables = AnyVariables,
>(
  addressUrl: string,
  apiKey: string,
  query: DocumentInput<T, Variables>,
  variables: Variables,
  signal: AbortSignal,
  handleResult: (v: OperationResult<T, Variables>) => void,
  context?: Partial<OperationContext> | undefined,
) {
  const client = createGraphqlClient(addressUrl, apiKey, signal);

  const operation = client.subscription<T, Variables>(
    query,
    variables,
    context,
  );

  let subscription = operation.subscribe(handleResult);

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
      subscription = operation.subscribe(handleResult);
    }
  };
  document.addEventListener("visibilitychange", handleVisibilityChange);

  signal.addEventListener("abort", () => {
    document.removeEventListener("visibilitychange", handleVisibilityChange);
    maybeClearTimeout();
    subscription.unsubscribe();
  });

  return subscription;
}
