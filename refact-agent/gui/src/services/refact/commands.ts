import { RootState } from "../../app/store";
import { parseOrElse } from "../../utils";
import { AT_COMMAND_COMPLETION, AT_COMMAND_PREVIEW } from "./consts";
import { type ChatContextFile } from "./types";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { callEngine } from "./call_engine";

export type CompletionArgs = {
  query: string;
  cursor: number;
  top_n?: number;
};

export const commandsApi = createApi({
  reducerPath: "commands",
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
    getCommandCompletion: builder.query<
      CommandCompletionResponse,
      CompletionArgs
    >({
      queryFn: async (args, api, _opts, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, AT_COMMAND_COMPLETION, {
            method: "POST",
            credentials: "same-origin",
            redirect: "follow",
            body: JSON.stringify({
              query: args.query,
              cursor: args.cursor,
              top_n: args.top_n ?? 5,
            }),
            headers: {
              "Content-Type": "application/json",
            },
          });

          const builtinCompletions =
            "@help".startsWith(args.query) && args.query.length !== 0
              ? ["@help"]
              : [];

          if (isCommandCompletionResponse(data)) {
            return {
              data: {
                ...data,
                completions: [...builtinCompletions, ...data.completions],
              },
            };
          } else if (isDetailMessage(data)) {
            return {
              data: {
                completions: [...builtinCompletions],
                replace: [0, 0],
                is_cmd_executable: false,
              },
            };
          } else {
            return {
              error: {
                error: "Invalid response from command completion",
                data: data,
                status: "CUSTOM_ERROR",
              },
            };
          }
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
    getCommandPreview: builder.query<(ChatContextFile | string)[], string>({
      queryFn: async (query, api, _opts, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, AT_COMMAND_PREVIEW, {
            method: "POST",
            credentials: "same-origin",
            redirect: "follow",
            body: JSON.stringify({ query }),
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isCommandPreviewResponse(data) && !isDetailMessage(data)) {
            return {
              error: {
                data: data,
                status: "CUSTOM_ERROR",
                error: "Invalid response from command preview",
              },
            };
          }

          if (isDetailMessage(data)) {
            return { data: [] };
          }

          const files = data.messages.reduce<(ChatContextFile | string)[]>(
            (acc, curr) => {
              if (curr.role === "context_file") {
                const fileData = parseOrElse<ChatContextFile[]>(curr.content, []);
                return [...acc, ...fileData];
              }
              return [...acc, curr.content];
            },
            []
          );

          return { data: files };
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
  refetchOnMountOrArgChange: true,
});

export type CommandCompletionResponse = {
  completions: string[];
  replace: [number, number];
  is_cmd_executable: boolean;
};

export function isCommandCompletionResponse(
  json: unknown,
): json is CommandCompletionResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("completions" in json)) return false;
  if (!("replace" in json)) return false;
  if (!("is_cmd_executable" in json)) return false;
  return true;
}

export type DetailMessage = {
  detail: string;
};

export function isDetailMessage(json: unknown): json is DetailMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("detail" in json)) return false;
  return true;
}

export type CommandPreviewContent = {
  content: string;
  role: "context_file" | "plain_text";
};

function isCommandPreviewContent(json: unknown): json is CommandPreviewContent {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("content" in json)) return false;
  if (typeof json.content !== "string") return false;
  if (!("role" in json)) return false;
  if (json.role === "context_file") return true;
  if (json.role === "plain_text") return true;
  return false;
}

export type CommandPreviewResponse = {
  messages: CommandPreviewContent[];
};

export function isCommandPreviewResponse(
  json: unknown,
): json is CommandPreviewResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("messages" in json)) return false;
  if (!Array.isArray(json.messages)) return false;
  if (!json.messages.length) return true;
  return json.messages.some(isCommandPreviewContent);
}