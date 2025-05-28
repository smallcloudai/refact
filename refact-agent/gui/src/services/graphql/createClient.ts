import {
  createClient,
  debugExchange,
  cacheExchange,
  fetchExchange,
  subscriptionExchange,
} from "@urql/core";
import { createClient as createWSClient } from "graphql-ws";
import { query } from "happy-dom/lib/PropertySymbol.js";
import { WebSocket } from "ws";
export { type Client } from "@urql/core";

const THREE_MINUTES = 3 * 60 * 1000;

export const createGraphqlClient = (apiKey: string, signal: AbortSignal) => {
  // const apiKey = "sk_alice_123456";
  // const baseUrl = "localhost:8008/v1/graphql";
  console.log("creating client");
  const baseUrl = "test-teams-v1.smallcloud.ai/v1/graphql";

  // TODO: should be secure by default
  const protocol = window.location.protocol === "https:" ? "https" : "http";
  const wsProtocol = window.location.protocol === "https:" ? "wss" : "ws";

  const wsClient = createWSClient({
    url: `${wsProtocol}://${baseUrl}`,
    connectionParams: { apikey: apiKey },
    webSocketImpl: WebSocket,
    retryAttempts: 5,
  });

  signal.addEventListener("abort", () => {
    console.log("aborting wsClient");
    void wsClient.dispose();
  });

  const urqlClient = createClient({
    url: `${protocol}://${baseUrl}`,
    exchanges: [
      // TODO: only enable this during development
      debugExchange,
      // cacheExchange,
      subscriptionExchange({
        forwardSubscription: (operation) => ({
          subscribe: (sink) => {
            const payload = { ...operation, query: operation.query ?? "" };
            const dispose = wsClient.subscribe(payload, sink);
            return { unsubscribe: dispose };
          },
          // subscribe: (sink) => ({
          //   unsubscribe: wsClient.subscribe(
          //     {
          //       ...operation,
          //       query:
          //         typeof operation.query === "string"
          //           ? operation.query
          //           : (() => {
          //               throw new Error(
          //                 "Subscription operation.query in undefined",
          //               );
          //             })(),
          //     },
          //     sink,
          //   ),
          // }),
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

  return urqlClient;
};
