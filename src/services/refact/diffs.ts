import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { PATCH_URL, APPLY_ALL_URL } from "./consts";
import { ChatMessages, DiffChunk, isDiffChunk } from "./types";
import { RootState } from "../../app/store";
import { createAction } from "@reduxjs/toolkit";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";

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

export type PatchResult = {
  file_text: string;
  already_applied: boolean;
  file_name_edit: string | null;
  file_name_delete: string | null;
  file_name_add: string | null;
};

function isPatchResult(json: unknown): json is PatchResult {
  if (!json || typeof json !== "object") return false;

  if (!("file_text" in json)) return false;
  if (typeof json.file_text !== "string") return false;

  if (!("already_applied" in json)) return false;
  if (typeof json.already_applied !== "boolean") return false;

  if (!("file_name_edit" in json)) return false;
  if (typeof json.file_name_edit !== "string" && json.file_name_edit !== null) {
    return false;
  }

  if (!("file_name_delete" in json)) return false;
  if (
    typeof json.file_name_delete !== "string" &&
    json.file_name_delete !== null
  ) {
    return false;
  }

  if (!("file_name_add" in json)) return false;
  if (typeof json.file_name_add !== "string" && json.file_name_add !== null) {
    return false;
  }

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
type ApplyAllResponse = {
  chunks: DiffChunk[];
};
function isApplyAllResponse(json: unknown): json is ApplyAllResponse {
  if (!json || typeof json !== "object") return false;
  if (!("chunks" in json)) return false;
  if (!Array.isArray(json.chunks)) return false;
  if (!json.chunks.every(isDiffChunk)) return false;
  return true;
}

type PatchRequest = {
  pin: string;
  messages: ChatMessages;
};

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
  endpoints: (builder) => ({
    patchSingleFileFromTicket: builder.mutation<PatchResponse, PatchRequest>({
      async queryFn(args, api, _extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PATCH_URL}`;

        const ticket = args.pin.split(" ")[1] ?? "";
        const messages = formatMessagesForLsp(args.messages);

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

    applyAllPatchesInMessages: builder.mutation<ApplyAllResponse, ChatMessages>(
      {
        async queryFn(messages, api, extraOptions, baseQuery) {
          const state = api.getState() as RootState;
          const port = state.config.lspPort as unknown as number;
          const url = `http://127.0.0.1:${port}${APPLY_ALL_URL}`;
          const formattedMessage = formatMessagesForLsp(messages);
          const result = await baseQuery({
            ...extraOptions,
            url,
            credentials: "same-origin",
            redirect: "follow",
            method: "POST",
            body: {
              messages: formattedMessage,
            },
          });

          if (result.error) {
            return { error: result.error };
          }

          if (!isApplyAllResponse(result.data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse apply all response",
                data: result.data,
              },
            };
          }

          return {
            data: result.data,
          };
        },
      },
    ),
  }),
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
