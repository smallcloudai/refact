import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import {
  DIFF_STATE_URL,
  DIFF_APPLY_URL,
  DIFF_PREVIEW_URL,
  PATCH_URL,
} from "./consts";
import {
  ChatMessages,
  DiffChunk,
  isDiffChunk,
  isDiffErrorResponseData,
} from "./types";
import { RootState } from "../../app/store";
import { createAction } from "@reduxjs/toolkit";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";

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

type PatchState = {
  chunk_id: number;
  applied: boolean;
  can_unapply: boolean;
  success: boolean;
  detail: null | string;
};

function isPatchState(json: unknown): json is PatchState {
  if (!json || typeof json !== "object") return false;
  if (!("chunk_id" in json)) return false;
  if (typeof json.chunk_id !== "number") return false;
  if (!("applied" in json)) return false;
  if (typeof json.applied !== "boolean") return false;
  if (!("can_unapply" in json)) return false;
  if (typeof json.can_unapply !== "boolean") return false;
  if (!("success" in json)) return false;
  if (typeof json.success !== "boolean") return false;
  return true;
}

type PatchResult = {
  file_text: string;
  file_name_edit: string | null;
  file_name_delete: string | null;
  file_name_add: string | null;
};

function isPatchResult(json: unknown): json is PatchResult {
  if (!json || typeof json !== "object") return false;
  if (!("file_text" in json)) return false;
  if (typeof json.file_text !== "string") return false;
  if (!("file_name_edit" in json)) return false;
  if (typeof json.file_name_edit !== "string" && json.file_name_edit !== null)
    return false;
  if (!("file_name_delete" in json)) return false;
  if (
    typeof json.file_name_delete !== "string" &&
    json.file_name_delete !== null
  )
    return false;
  if (!("file_name_add" in json)) return false;
  if (typeof json.file_name_add !== "string" && json.file_name_add !== null)
    return false;
  return true;
}

type PatchResponse = {
  state: PatchState[];
  results: PatchResult[];
  chunks: DiffChunk[];
};

function isPatchResponse(json: unknown): json is PatchResponse {
  if (!json || typeof json !== "object") return false;
  if (!("state" in json)) return false;
  if (!Array.isArray(json.state)) return false;
  if (!json.state.every(isPatchState)) return false;
  if (!("results" in json)) return false;
  if (!Array.isArray(json.results)) return false;
  if (!json.results.every(isPatchResult)) return false;
  if (!("chunks" in json)) return false;
  if (!Array.isArray(json.chunks)) return false;
  if (!json.chunks.every(isDiffChunk)) return false;
  return true;
}

type PatchRequest = {
  pin: string;
  messages: ChatMessages;
};

// function cacheIdForChunk(chunk: DiffChunk) {
//   return `${chunk.file_name}-${chunk.line1}-${chunk.line2}`;
// }

export const resetDiffApi = createAction("diffs/reset");

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
  tagTypes: ["DIFF_STATE", "DIFF_PREVIEW", "DIFF_PATCH"],
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
        return [{ type: "DIFF_STATE" }, { type: "DIFF_PATCH" }];
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

    patchSingleFileFromTicket: builder.query<PatchResponse, PatchRequest>({
      providesTags: ["DIFF_PATCH"],
      async queryFn(args, api, _extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PATCH_URL}`;

        const ticket = args.pin.split(" ")[1] ?? "";
        const messages = formatMessagesForLsp(args.messages);
        // const messages = [
        //   { role: "assistant", content: args.pin + "\n" + args.markdown },
        // ];

        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
          method: "POST",
          body: {
            messages,
            ticket_ids: [ticket],
          },
        });

        if (result.error) return { error: result.error };

        if (!isPatchResponse(result.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: "Failed to parse patch response",
              data: result.data,
            },
          };
        }

        return { data: result.data };
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
    file_name_edit: string | null;
    file_name_delete: null | string;
    file_name_add: null | string;
  }[];
}
