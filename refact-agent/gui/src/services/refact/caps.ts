import { RootState } from "../../app/store";
import { CAPS_URL } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { CodeChatModel, CodeCompletionModel, EmbeddingModel } from "./models";

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

        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (!isCapsResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from caps",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
  }),
  refetchOnMountOrArgChange: true,
});

export const capsEndpoints = capsApi.endpoints;

// Export the generated RTK Query hook
export const { useGetCapsQuery } = capsApi;

export type CapCost = {
  prompt: number;
  generated: number;
  cache_read?: number;
  cache_creation?: number;
};

function isCapCost(json: unknown): json is CapCost {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("prompt" in json)) return false;
  if (typeof json.prompt !== "number") return false;
  if (!("generated" in json)) return false;
  if (typeof json.generated !== "number") return false;
  return true;
}
type CapsMetadata = {
  pricing?: Record<string, CapCost>;
  features?: string[];
};

function isCapsMetadata(json: unknown): json is CapsMetadata {
  if (json === null) return true;
  if (typeof json !== "object") return false;
  if ("pricing" in json && json.pricing) {
    return Object.values(json.pricing).every(isCapCost);
  }
  return true;
}

export type CapsResponse = {
  caps_version: number;
  cloud_name: string;

  chat_default_model: string;
  chat_models: Record<string, CodeChatModel>;
  code_chat_default_system_prompt: string;
  completion_models: Record<string, CodeCompletionModel>;
  completion_default_model: string;
  code_completion_n_ctx: number;
  embedding_model?: EmbeddingModel;
  chat_thinking_model: string;
  chat_light_model: string;

  endpoint_chat_passthrough: string;
  endpoint_style: string;
  endpoint_template: string;
  running_models: string[];
  telemetry_basic_dest: string;
  tokenizer_path_template: string;
  telemetry_basic_retrieve_my_own: string;
  tokenizer_rewrite_path: Record<string, unknown>;
  support_metadata: boolean;
  metadata: CapsMetadata | null;
  customization: string;
};

export function isCapsResponse(json: unknown): json is CapsResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("metadata" in json)) return false;
  if (!isCapsMetadata(json.metadata)) return false;
  if (!("chat_default_model" in json)) return false;
  if (typeof json.chat_default_model !== "string") return false;
  if (!("chat_models" in json)) return false;
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
