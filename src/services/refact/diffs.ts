import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { getApiKey, parseOrElse } from "../../utils";
import { DEFF_STATE_URL, DIFF_APPLY_URL } from "./consts";
import { DiffChunk } from "./types";

type DiffAppliedStateArgs = {
  chunks: DiffChunk[];
  toolCallId: string;
};

type DiffOperationArgs = {
  chunks: DiffChunk[];
  toApply: boolean[];
  toolCallId: string;
};
export const diffApi = createApi({
  reducerPath: "diffs",
  baseQuery: fetchBaseQuery({
    baseUrl: "http://localhost:8001",
  }),
  tagTypes: ["diffs"],
  endpoints: (builder) => ({
    diffState: builder.query<DiffAppliedStateResponse, DiffAppliedStateArgs>({
      query: ({ chunks }) => ({
        url: DEFF_STATE_URL,
        method: "POST",
        credentials: "same-origin",
        redirect: "follow",
        body: { chunks },
      }),

      providesTags: (_result, _error, args) => {
        return [{ type: "diffs", id: args.toolCallId }];
      },
      transformResponse: (response: unknown) => {
        // TODO: type check
        return response as DiffAppliedStateResponse;
      },
    }),
    diffApply: builder.mutation<DiffOperationResponse, DiffOperationArgs>({
      query: ({ chunks, toApply }) => ({
        url: DIFF_APPLY_URL,
        method: "POST",
        body: { chunks, apply: toApply },
      }),
      transformResponse: (response: unknown) => {
        // TODO: type check
        return response as DiffOperationResponse;
      },
      invalidatesTags: (_result, _error, args) => {
        return [{ type: "diffs", id: args.toolCallId }];
      },
    }),
  }),
});

export interface DiffAppliedStateResponse {
  id: number;
  state: boolean[];
  can_apply: boolean[];
}

export async function checkDiff(
  chunks: DiffChunk[],
  lspUrl?: string,
): Promise<DiffAppliedStateResponse> {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DEFF_STATE_URL}`
    : DEFF_STATE_URL;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify({ chunks }),
    credentials: "same-origin",
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    headers: {
      accept: "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const text = await response.text();

  const json = parseOrElse<DiffAppliedStateResponse>(text, {
    id: 0,
    state: [],
    can_apply: [],
  });

  return json;
}

export interface DiffOperationResponse {
  fuzzy_results: {
    chunk_id: number;
    fuzzy_n_used: number;
  }[];

  state: (0 | 1 | 2)[];
}
// TODO: delete this
export async function doDiff(
  chunks: DiffChunk[],
  toApply: boolean[],
  lspUrl?: string,
): Promise<DiffOperationResponse> {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DIFF_APPLY_URL}`
    : DIFF_APPLY_URL;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify({
      apply: toApply,
      chunks,
    }),
    credentials: "same-origin",
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    headers: {
      accept: "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const text = await response.text();

  const json = parseOrElse<DiffOperationResponse>(text, {
    fuzzy_results: [],
    state: [],
  });

  return json;
}
