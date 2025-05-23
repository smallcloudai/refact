import {
  createClient,
  debugExchange,
  cacheExchange,
  fetchExchange,
  subscriptionExchange,
} from "urql";
import { createClient as createWSClient } from "graphql-ws";
import { WebSocket } from "ws";

const baseUrl = "localhost:8008/v1/graphql";
const apiKey = "sk_alice_123456";

const wsClient = createWSClient({
  url: `ws://${baseUrl}`,
  connectionParams: {
    apikey: apiKey,
  },
  webSocketImpl: WebSocket,
  retryAttempts: 5,
});

export const urqlClient = createClient({
  url: `https://${baseUrl}`,
  exchanges: [
    debugExchange,
    cacheExchange,
    subscriptionExchange({
      forwardSubscription: (operation) => ({
        subscribe: (sink) => ({
          unsubscribe: wsClient.subscribe(
            {
              ...operation,
              query:
                typeof operation.query === "string"
                  ? operation.query
                  : (() => {
                      throw new Error(
                        "Subscription operation.query in undefined",
                      );
                    })(),
            },
            sink,
          ),
        }),
      }),
    }),
    fetchExchange,
  ],
  fetchOptions: () => ({
    headers: {
      Authorization: `Bearer ${apiKey}`,
    },
  }),
});
