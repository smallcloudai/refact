import { RootState } from "../../app/store";
import {
  AT_TOOLS_AVAILABLE_URL,
  TOOLS_CHECK_CONFIRMATION,
  EDIT_TOOL_DRY_RUN_URL,
} from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { ChatMessage, DiffChunk, isDiffChunk, ToolCall } from "./types";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";
import { callEngine } from "./call_engine";

export const toolsApi = createApi({
  reducerPath: "tools",
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
    getTools: builder.query<ToolCommand[], undefined>({
      queryFn: async (_args, api, _extraOptions, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, AT_TOOLS_AVAILABLE_URL, {
            credentials: "same-origin",
            redirect: "follow",
          });

          if (!Array.isArray(data)) {
            return {
              error: {
                error: "Invalid response from tools",
                data: data,
                status: "CUSTOM_ERROR",
              },
            };
          }

          const tools = data.filter((d) => isToolCommand(d)) as ToolCommand[];
          return { data: tools };
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
    checkForConfirmation: builder.mutation<
      ToolConfirmationResponse,
      ToolConfirmationRequest
    >({
      queryFn: async (args, api, _extraOptions, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const { messages, tool_calls } = args;
          const messagesForLsp = formatMessagesForLsp(messages);

          const data = await callEngine<unknown>(state, TOOLS_CHECK_CONFIRMATION, {
            method: "POST",
            body: JSON.stringify({
              tool_calls: tool_calls,
              messages: messagesForLsp,
            }),
            credentials: "same-origin",
            redirect: "follow",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isToolConfirmationResponse(data)) {
            return {
              error: {
                error: "Invalid response from tools",
                data: data,
                status: "CUSTOM_ERROR",
              },
            };
          }

          return { data };
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
    dryRunForEditTool: builder.mutation<
      ToolEditResult,
      { toolName: string; toolArgs: Record<string, unknown> }
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;

          const data = await callEngine<unknown>(state, EDIT_TOOL_DRY_RUN_URL, {
            method: "POST",
            body: JSON.stringify({
              tool_name: args.toolName,
              tool_args: args.toolArgs,
            }),
            credentials: "same-origin",
            redirect: "follow",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isToolEditResult(data)) {
            return {
              error: {
                error: `Invalid response from ${EDIT_TOOL_DRY_RUN_URL}`,
                data: data,
                status: "CUSTOM_ERROR",
              },
            };
          }

          return { data };
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

export type ToolParams = {
  name: string;
  type: string;
  description: string;
};

export type ToolFunction = {
  agentic?: boolean;
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  parameters_required?: string[];
};

export type ToolCommand = {
  function: ToolFunction;
  type: "function";
};

export type ToolConfirmationPauseReason = {
  type: "confirmation" | "denial";
  command: string;
  rule: string;
  tool_call_id: string;
  integr_config_path: string | null;
};

export type ToolConfirmationResponse = {
  pause: boolean;
  pause_reasons: ToolConfirmationPauseReason[];
};

export type ToolConfirmationRequest = {
  tool_calls: ToolCall[];
  messages: ChatMessage[];
};

function isToolCommand(tool: unknown): tool is ToolCommand {
  if (!tool) return false;
  if (typeof tool !== "object") return false;
  if (!("type" in tool) || !("function" in tool)) return false;
  return true;
}

export function isToolConfirmationResponse(
  data: unknown,
): data is ToolConfirmationResponse {
  if (!data) return false;
  if (typeof data !== "object") return false;
  const response = data as ToolConfirmationResponse;
  if (typeof response.pause !== "boolean") return false;
  if (!Array.isArray(response.pause_reasons)) return false;
  for (const reason of response.pause_reasons) {
    if (typeof reason.type !== "string") return false;
    if (typeof reason.command !== "string") return false;
    if (typeof reason.rule !== "string") return false;
    if (typeof reason.tool_call_id !== "string") return false;
  }
  return true;
}

export type ToolEditResult = {
  file_before: string;
  file_after: string;
  chunks: DiffChunk[];
};

export function isToolEditResult(data: unknown): data is ToolEditResult {
  if (!data) return false;
  if (typeof data !== "object") return false;
  if (!("file_before" in data)) return false;
  if (typeof data.file_before !== "string") return false;
  if (!("file_after" in data)) return false;
  if (typeof data.file_after !== "string") return false;
  if (!("chunks" in data)) return false;
  if (!Array.isArray(data.chunks)) return false;

  return data.chunks.every(isDiffChunk);
}