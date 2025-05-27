import {
  Provider,
  createClient,
  debugExchange,
  cacheExchange,
  fetchExchange,
  subscriptionExchange,
} from "urql";
import { createClient as createWSClient } from "graphql-ws";
import { WebSocket } from "ws";
import React, { useMemo } from "react";
import { useAppSelector } from "./src/hooks";
import { selectConfig } from "./src/features/Config/configSlice";

export const UrqlProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  // const apiKey = useAppSelector(selectConfig).apiKey;
  const apiKey = "sk_alice_123456";
  const baseUrl = "localhost:8008/v1/graphql";

  const wsClient = useMemo(
    () =>
      createWSClient({
        url: `ws://${baseUrl}`,
        connectionParams: { apikey: apiKey },
        webSocketImpl: WebSocket,
        retryAttempts: 5,
      }),
    [baseUrl, apiKey],
  );

  const urqlClient = useMemo(
    () =>
      createClient({
        url: `http://${baseUrl}`,
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
      }),
    [baseUrl, apiKey, wsClient],
  );

  return <Provider value={urqlClient}>{children}</Provider>;
};
