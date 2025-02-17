import { RootState } from "../../app/store";
import { TELEMETRY_CHAT_PATH, TELEMETRY_NET_PATH } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

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
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${TELEMETRY_CHAT_PATH}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: arg,
        });

        if (response.error) {
          const netWorkErrorResponse = await baseQuery({
            ...extraOptions,
            url: `http://127.0.0.1:${port}${TELEMETRY_NET_PATH}`,
            method: "POST",
            body: { ...arg, url: TELEMETRY_NET_PATH },
          });
          return { data: netWorkErrorResponse.data };
        }

        return { data: response.data };
      },
    }),
    sendTelemetryNetEvent: builder.query<unknown, TelemetryNetEvent>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const netWorkErrorResponse = await baseQuery({
          ...extraOptions,
          url: `http://127.0.0.1:${port}${TELEMETRY_NET_PATH}`,
          method: "POST",
          body: { ...arg },
        });
        return { data: netWorkErrorResponse.data };
      },
    }),
  }),
});
