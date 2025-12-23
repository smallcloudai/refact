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
      async queryFn(args, api, _extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const { checkpoints } = args;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PREVIEW_CHECKPOINTS}`;

        const chat_id = state.chat.thread.id;
        const mode = state.chat.thread.mode;

        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
          method: "POST",
          body: {
            meta: {
              chat_id,
              chat_mode: mode ?? "EXPLORE",
            },
            checkpoints,
          },
        });

        if (result.error) return { error: result.error };

        if (!isPreviewCheckpointsResponse(result.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: "Failed to parse preview checkpoints response",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),
    restoreCheckpoints: builder.mutation<
      RestoreCheckpointsResponse,
      RestoreCheckpointsPayload
    >({
      async queryFn(args, api, _extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const { checkpoints } = args;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${RESTORE_CHECKPOINTS}`;

        const chat_id = state.chat.thread.id;
        const mode = state.chat.thread.mode;

        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
          method: "POST",
          body: {
            meta: {
              chat_id,
              chat_mode: mode ?? "EXPLORE",
            },
            checkpoints,
          },
        });

        if (result.error) return { error: result.error };
        if (!isRestoreCheckpointsResponse(result.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: "Failed to parse restored checkpoints response",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),
  }),
});
