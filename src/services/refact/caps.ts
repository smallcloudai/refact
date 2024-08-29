import { RootState } from "../../app/store";
import { CAPS_URL } from "./consts";
import {
  createApi,
  fetchBaseQuery,
  FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";

export const capsApi = createApi({
  reducerPath: "caps",
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
    getCaps: builder.query<CapsResponse, undefined>({
      queryFn: async (_args, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${CAPS_URL}`;
        // return baseQuery(url);
        const result = await baseQuery(url);
        if (result.error) {
          return { error: result.error };
        }
        if (!isCapsResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from caps",
              data: result.data,
            } as FetchBaseQueryError,
          };
        }

        return { data: result.data };
      },
    }),
  }),
});

export const capsEndpoints = capsApi.endpoints;

export type CodeChatModel = {
  default_scratchpad: string;
  n_ctx: number;
  similar_models: string[];
  supports_tools?: boolean | null | undefined;
  supports_scratchpads: Record<
    string,
    {
      default_system_message: string;
    }
  >;
};

export type CodeCompletionModel = {
  default_scratchpad: string;
  n_ctx: number;
  similar_models: string[];
  supports_scratchpads: Record<string, Record<string, unknown>>;
};

export type CapsResponse = {
  caps_version: number;
  cloud_name: string;
  code_chat_default_model: string;
  code_chat_models: Record<string, CodeChatModel>;
  code_completion_default_model: string;
  code_completion_models: Record<string, CodeCompletionModel>;
  code_completion_n_ctx: number;
  endpoint_chat_passthrough: string;
  endpoint_style: string;
  endpoint_template: string;
  running_models: string[];
  telemetry_basic_dest: string;
  telemetry_corrected_snippets_dest: string;
  tokenizer_path_template: string;
  tokenizer_rewrite_path: Record<string, unknown>;
};

export function isCapsResponse(json: unknown): json is CapsResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("code_chat_default_model" in json)) return false;
  if (typeof json.code_chat_default_model !== "string") return false;
  if (!("code_chat_models" in json)) return false;
  return true;
}

type CapsErrorResponse = {
  detail: string;
};

export function isCapsErrorResponse(json: unknown): json is CapsErrorResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("detail" in json)) return false;
  if (typeof json.detail !== "string") return false;
  return true;
}

// export async function getCaps(lspUrl?: string): Promise<CapsResponse> {
//   const capsEndpoint = lspUrl
//     ? `${lspUrl.replace(/\/*$/, "")}${CAPS_URL}`
//     : CAPS_URL;

//   const response = await fetch(capsEndpoint, {
//     method: "GET",
//     credentials: "same-origin",
//     headers: {
//       accept: "application/json",
//     },
//   });

//   if (!response.ok) {
//     throw new Error(response.statusText);
//   }

//   const json: unknown = await response.json();

//   if (!isCapsResponse(json)) {
//     throw new Error("Invalid response from caps");
//   }

//   return json;
// }
