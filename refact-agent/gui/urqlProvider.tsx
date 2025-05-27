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
  // const apiKey = "sk_alice_123456";
  const apiKey = useAppSelector(selectConfig).apiKey;
  // const baseUrl = "localhost:8008/v1/graphql";
  const baseUrl = "test-teams-v1.smallcloud.ai/v1/graphql";

  const protocol = window.location.protocol === "https:" ? "https" : "http";
  const wsProtocol = window.location.protocol === "https:" ? "wss" : "ws";

  const wsClient = useMemo(
    () =>
      createWSClient({
        url: `${wsProtocol}://${baseUrl}`,
        connectionParams: { apikey: apiKey },
        webSocketImpl: WebSocket,
        retryAttempts: 5,
      }),
    [baseUrl, apiKey, wsProtocol],
  );

  const urqlClient = useMemo(
    () =>
      createClient({
        url: `${protocol}://${baseUrl}`,
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
    [baseUrl, apiKey, wsClient, protocol],
  );

  return <Provider value={urqlClient}>{children}</Provider>;
};
