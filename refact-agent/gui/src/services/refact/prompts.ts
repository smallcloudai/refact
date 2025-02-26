import { RootState } from "../../app/store";
import { CUSTOM_PROMPTS_URL } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { callEngine } from "./call_engine";

export const promptsApi = createApi({
  reducerPath: "prompts",
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
    getPrompts: builder.query<SystemPrompts, undefined>({
      queryFn: async (_args, api, _opts, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, CUSTOM_PROMPTS_URL, {
            credentials: "same-origin",
            redirect: "follow",
          });

          if (!isCustomPromptsResponse(data)) {
            return {
              error: {
                data: data,
                error: "Invalid response from server",
                status: "CUSTOM_ERROR",
              },
            };
          }

          return { data: data.system_prompts };
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

export const promptsEndpoints = promptsApi.endpoints;

export type SystemPrompt = {
  text: string;
  description: string;
};

function isSystemPrompt(json: unknown): json is SystemPrompt {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("text" in json)) return false;
  if (!("description" in json)) return false;
  return true;
}

export type SystemPrompts = Record<string, SystemPrompt>;

export function isSystemPrompts(json: unknown): json is SystemPrompts {
  if (!json) return false;
  if (typeof json !== "object") return false;
  for (const value of Object.values(json)) {
    if (!isSystemPrompt(value)) return false;
  }
  return true;
}

export type CustomPromptsResponse = {
  system_prompts: SystemPrompts;
  toolbox_commands: Record<string, unknown>;
};

export function isCustomPromptsResponse(
  json: unknown,
): json is CustomPromptsResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("system_prompts" in json)) return false;
  if (typeof json.system_prompts !== "object") return false;
  if (json.system_prompts === null) return false;
  return isSystemPrompts(json.system_prompts);
}