import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { DIFF_STATE_URL, DIFF_APPLY_URL, DIFF_PREVIEW_URL } from "./consts";
import { DiffChunk, isDiffErrorResponseData } from "./types";
import { RootState } from "../../app/store";

export type DiffAppliedStateArgs = {
  chunks: DiffChunk[];
};

export type DiffOperationArgs = {
  chunks: DiffChunk[];
  toApply: boolean[];
  toolCallId?: string;
};

export type DiffPreviewArgs = {
  chunks: DiffChunk[];
  toApply: boolean[];
};

export interface DiffAppliedStateResponse {
  id: number;
  state: boolean[];
  can_apply: boolean[];
}

function isDiffAppliedStateResponse(
  json: unknown,
): json is DiffAppliedStateResponse {
  if (json === null) return false;
  if (typeof json !== "object") return false;
  if (!("id" in json)) return false;
  if (!("state" in json)) return false;
  if (!("can_apply" in json)) return false;
  return true;
}

export interface DiffStateResponse {
  state: boolean;
  can_apply: boolean;
  chunk: DiffChunk;
}

export type DiffApplyManyArgs = {
  diffs: {
    tool_call_id: string;
    chunks: DiffChunk[];
  };
  toApply: boolean;
}[];

// function cacheIdForChunk(chunk: DiffChunk) {
//   return `${chunk.file_name}-${chunk.line1}-${chunk.line2}`;
// }

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
  tagTypes: ["DIFF_STATE", "DIFF_PREVIEW"],
  endpoints: (builder) => ({
    diffState: builder.query<DiffStateResponse[], DiffAppliedStateArgs>({
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
        const { data } = result;
        if (!isDiffAppliedStateResponse(data)) {
          return {
            error: {
              error: "invalid response for diff state",
              status: "CUSTOM_ERROR",
              data,
            },
          };
        }

        const merged = args.chunks.map<DiffStateResponse>((chunk, index) => {
          return {
            chunk,
            state: data.state[index] ?? false,
            can_apply: data.can_apply[index] ?? false,
          };
        });
        return { data: merged };
      },
      providesTags: (_result, _error, _args) => {
        // TODO: this could be more efficient
        return [{ type: "DIFF_STATE" }];
      },
    }),
    diffApply: builder.mutation<
      DiffOperationResponse | DiffApplyErrorResponse[],
      DiffOperationArgs
    >({
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

        if (Array.isArray(result.data)) {
          const maybeErrorChunks = result.data.filter(isDiffErrorResponseData);
          if (maybeErrorChunks.length > 0) {
            return { data: maybeErrorChunks };
          }
        }

        return { data: result.data as DiffOperationResponse };
      },
      invalidatesTags: (_result, _error, _args) => {
        // TODO: this could be more efficient
        return [{ type: "DIFF_STATE" }];
      },
    }),

    diffPreview: builder.query<DiffPreviewResponse, DiffPreviewArgs>({
      queryFn: async (args, api, _extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DIFF_PREVIEW_URL}`;
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

        return { data: result.data as DiffPreviewResponse };
      },
      // providesTags: (_result, _error, args) => {
      //   return [{ type: "DIFF_PREVIEW", id: args.toolCallId }];
      // },
      // invalidatesTags: (res, _error) => {
      //   if (!res) return [];
      //   return res.state.map((chunk) => {
      //     return { type: "DIFF_STATE", id: chunk.chunk_id };
      //   });
      // },
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

export type DiffApplyErrorResponse = {
  chunk_id: number;
  applied: false;
  can_unapply: false;
  success: false;
  detail: null | string;
};

export interface DiffPreviewResponse {
  state: DiffApplyResponse;
  results: {
    file_text: string;
    file_name_edit: string;
    file_name_delete: null | string;
    file_name_add: null | string;
  }[];
}
