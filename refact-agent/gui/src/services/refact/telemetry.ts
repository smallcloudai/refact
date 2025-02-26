import { RootState } from "../../app/store";
import { TELEMETRY_CHAT_PATH, TELEMETRY_NET_PATH } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { callEngine } from "./call_engine";

export type TelemetryChatEvent = {
  scope: string;
  success: boolean;
  error_message: string;
};

export type TelemetryNetEvent = {
  url: string; // relative path
  scope: string;
  success: boolean;
  error_message: string;
};

export type TelemetryNetworkEvent = TelemetryChatEvent & { url: string };

export const telemetryApi = createApi({
  reducerPath: "telemetryApi",
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, { getState }) => {
      const token = (getState() as RootState).config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    sendTelemetryChatEvent: builder.query<unknown, TelemetryChatEvent>({
      async queryFn(arg, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const response = await callEngine<unknown>(state, TELEMETRY_CHAT_PATH, {
            method: "POST",
            body: JSON.stringify(arg),
            headers: {
              "Content-Type": "application/json",
            },
          });

          return { data: response };
        } catch (error) {
          // If chat telemetry fails, try to send network error telemetry
          try {
            const state = api.getState() as RootState;
            const netWorkErrorResponse = await callEngine<unknown>(state, TELEMETRY_NET_PATH, {
              method: "POST",
              body: JSON.stringify({ ...arg, url: TELEMETRY_NET_PATH }),
              headers: {
                "Content-Type": "application/json",
              },
            });
            return { data: netWorkErrorResponse };
          } catch (netError) {
            return {
              error: {
                status: "FETCH_ERROR",
                error: String(netError),
              },
            };
          }
        }
      },
    }),
    sendTelemetryNetEvent: builder.query<unknown, TelemetryNetEvent>({
      async queryFn(arg, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const response = await callEngine<unknown>(state, TELEMETRY_NET_PATH, {
            method: "POST",
            body: JSON.stringify(arg),
            headers: {
              "Content-Type": "application/json",
            },
          });
          return { data: response };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
  }),
});