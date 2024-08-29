import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { DIFF_STATE_URL, DIFF_APPLY_URL } from "./consts";
import { DiffChunk } from "./types";
import { RootState } from "../../app/store";

export type DiffAppliedStateArgs = {
  chunks: DiffChunk[];
  toolCallId: string;
};

export type DiffOperationArgs = {
  chunks: DiffChunk[];
  toApply: boolean[];
  toolCallId: string;
};

export interface DiffAppliedStateResponse {
  id: number;
  state: boolean[];
  can_apply: boolean[];
}

export const diffApi = createApi({
  reducerPath: "diffs",
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, api) => {
      const getState = api.getState as () => RootState;
      const state = getState();
      const token = state.config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  tagTypes: ["diffs"],
  endpoints: (builder) => ({
    diffState: builder.query<DiffAppliedStateResponse, DiffAppliedStateArgs>({
      queryFn: async (args, api, _extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DIFF_STATE_URL}`;
        const result = await baseQuery({
          url: url,
          method: "POST",
          credentials: "same-origin",
          redirect: "follow",
          // referrer: "no-referrer",
          body: { chunks: args.chunks },
        });

        if (result.error) return { error: result.error };
        // TODO: type check
        return { data: result.data as DiffAppliedStateResponse };
      },
      providesTags: (_result, _error, args) => {
        return [{ type: "diffs", id: args.toolCallId }];
      },
    }),
    diffApply: builder.mutation<DiffOperationResponse, DiffOperationArgs>({
      queryFn: async (args, api, _extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DIFF_APPLY_URL}`;
        const result = await baseQuery({
          url: url,
          method: "POST",
          credentials: "same-origin",
          redirect: "follow",
          body: { chunks: args.chunks, apply: args.toApply },
        });

        if (result.error) {
          return { error: result.error };
        }

        return { data: result.data as DiffOperationResponse };
      },
      invalidatesTags: (_result, _error, args) => {
        return [{ type: "diffs", id: args.toolCallId }];
      },
    }),
  }),
  refetchOnMountOrArgChange: true,
});

export interface DiffOperationResponse {
  fuzzy_results: {
    chunk_id: number;
    fuzzy_n_used: number;
  }[];

  state: (0 | 1 | 2)[];
}

export type DiffApplyResponse = {
  chunk_id: number;
  applied: boolean;
  can_unapply: boolean;
  success: boolean;
  detail: null | string;
}[];

export interface DiffPreviewResponse {
  state: DiffApplyResponse;
  results: {
    file_text: string;
    file_name_edit: string;
    file_name_delete: null | string;
    file_name_add: null | string;
  }[];
}
