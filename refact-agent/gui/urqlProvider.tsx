import {
  Provider,
  createClient,
  cacheExchange,
  fetchExchange,
  // debugExchange,
  subscriptionExchange,
} from "urql";
import { createClient as createWSClient } from "graphql-ws";
import { WebSocket } from "ws";
import React, { useMemo } from "react";
import { useAppSelector } from "./src/hooks/useAppSelector";
import { selectConfig } from "./src/features/Config/configSlice";

export const UrqlProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const apiKey = useAppSelector(selectConfig).apiKey;
  const configUrl = useAppSelector(selectConfig).addressURL;
  const addressUrl =
    !configUrl || configUrl === "Refact" ? `https://app.refact.ai` : configUrl;

  const httpUrl = new URL(addressUrl);
  httpUrl.pathname = "/v1/graphql";
  const httpUrlString = useMemo(() => {
    const httpUrl = new URL(addressUrl);
    httpUrl.pathname = "/v1/graphql";
    return httpUrl.toString();
  }, [addressUrl]);

  const wsUrLString = useMemo(() => {
    const wsUrl = new URL(addressUrl);
    wsUrl.protocol = addressUrl.startsWith("http://") ? "ws" : "wss";
    wsUrl.pathname = "/v1/graphql";
    return wsUrl.toString();
  }, [addressUrl]);

  const wsClient = useMemo(
    () =>
      createWSClient({
        url: wsUrLString,
        connectionParams: { apikey: apiKey },
        webSocketImpl: WebSocket,
        retryAttempts: 5,
      }),
    [apiKey, wsUrLString],
  );

  const urqlClient = useMemo(
    () =>
      createClient({
        url: httpUrlString,
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
    [httpUrlString, wsClient, apiKey],
  );

  return <Provider value={urqlClient}>{children}</Provider>;
};
