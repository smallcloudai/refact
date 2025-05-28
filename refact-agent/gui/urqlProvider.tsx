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
  const apiKey = useAppSelector(selectConfig).apiKey;
  const baseUrl = "test-teams-v1.smallcloud.ai/v1/graphql";

  const protocol = "https";
  const wsProtocol = "wss";

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
          // TODO: only enable this during development
          // debugExchange,
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
