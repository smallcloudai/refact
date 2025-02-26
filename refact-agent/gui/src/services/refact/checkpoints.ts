import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";
import { PREVIEW_CHECKPOINTS, RESTORE_CHECKPOINTS } from "./consts";
import {
  isPreviewCheckpointsResponse,
  isRestoreCheckpointsResponse,
  PreviewCheckpointsPayload,
  PreviewCheckpointsResponse,
  RestoreCheckpointsPayload,
  RestoreCheckpointsResponse,
} from "../../features/Checkpoints/types";
import { callEngine } from "./call_engine";

export const checkpointsApi = createApi({
  reducerPath: "checkpointsApi",
  tagTypes: ["CHECKPOINTS"],
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, api) => {
      const getState = api.getState as () => RootState;
      const state = getState();
      const token = state.config.apiKey;
      headers.set("credentials", "same-origin");
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    previewCheckpoints: builder.mutation<
      PreviewCheckpointsResponse,
      PreviewCheckpointsPayload
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const { checkpoints } = args;
          const chat_id = state.chat.thread.id;
          const mode = state.chat.thread.mode;

          const data = await callEngine<unknown>(state, PREVIEW_CHECKPOINTS, {
            method: "POST",
            credentials: "same-origin",
            redirect: "follow",
            body: JSON.stringify({
              meta: {
                chat_id,
                chat_mode: mode ?? "EXPLORE",
              },
              checkpoints,
            }),
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isPreviewCheckpointsResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse preview checkpoints response",
                data: data,
              },
            };
          }

          return { data };
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
    restoreCheckpoints: builder.mutation<
      RestoreCheckpointsResponse,
      RestoreCheckpointsPayload
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const { checkpoints } = args;
          const chat_id = state.chat.thread.id;
          const mode = state.chat.thread.mode;

          const data = await callEngine<unknown>(state, RESTORE_CHECKPOINTS, {
            method: "POST",
            credentials: "same-origin",
            redirect: "follow",
            body: JSON.stringify({
              meta: {
                chat_id,
                chat_mode: mode ?? "EXPLORE",
              },
              checkpoints,
            }),
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isRestoreCheckpointsResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse restored checkpoints response",
                data: data,
              },
            };
          }

          return { data };
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