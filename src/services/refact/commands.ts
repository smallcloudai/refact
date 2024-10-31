import { RootState } from "../../app/store";
import { parseOrElse } from "../../utils";
import { AT_COMMAND_COMPLETION, AT_COMMAND_PREVIEW } from "./consts";
import { type ChatContextFile } from "./types";

import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

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
      queryFn: async (args, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${AT_COMMAND_COMPLETION}`;
        const response = await baseQuery({
          url,
          method: "POST",
          credentials: "same-origin",
          redirect: "follow",
          body: {
            query: args.query,
            cursor: args.cursor,
            top_n: args.top_n ?? 5,
          },
        });

        const builtinCompletions =
          "@help".startsWith(args.query) && args.query.length !== 0
            ? ["@help"]
            : [];

        if (response.error) return { error: response.error };
        if (isCommandCompletionResponse(response.data)) {
          return {
            data: {
              ...response.data,
              completions: [
                ...builtinCompletions,
                ...response.data.completions,
              ],
            },
          };
        } else if (isDetailMessage(response.data)) {
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
              data: response.data,
              status: "CUSTOM_ERROR",
            },
          };
        }
      },
    }),
    getCommandPreview: builder.query<(ChatContextFile | string)[], string>({
      queryFn: async (query, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${AT_COMMAND_PREVIEW}`;
        const response = await baseQuery({
          url,
          method: "POST",
          credentials: "same-origin",
          redirect: "follow",
          body: { query },
        });

        if (response.error) return { error: response.error };

        if (
          !isCommandPreviewResponse(response.data) &&
          !isDetailMessage(response.data)
        ) {
          return {
            error: {
              data: response.data,
              status: "CUSTOM_ERROR",
              error: "Invalid response from command preview",
            },
          };
        }

        if (isDetailMessage(response.data)) {
          return { data: [] };
        }

        const files = response.data.messages.reduce<
          (ChatContextFile | string)[]
        >((acc, curr) => {
          if (curr.role === "context_file") {
            const fileData = parseOrElse<ChatContextFile[]>(curr.content, []);
            return [...acc, ...fileData];
          }
          return [...acc, curr.content];
        }, []);

        return { data: files };
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
