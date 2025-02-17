import { RootState } from "../../app/store";
import { AT_TOOLS_AVAILABLE_URL, TOOLS_CHECK_CONFIRMATION } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { ChatMessage, ToolCall } from "./types";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";

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
      queryFn: async (_args, api, _extraOptions, baseQuery) => {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;
        const url = `http://127.0.0.1:${port}${AT_TOOLS_AVAILABLE_URL}`;
        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) return result;
        if (!Array.isArray(result.data)) {
          return {
            error: {
              error: "Invalid response from tools",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }
        const tools = result.data.filter((d) =>
          isToolCommand(d),
        ) as ToolCommand[];
        return { data: tools };
      },
    }),
    checkForConfirmation: builder.mutation<
      ToolConfirmationResponse,
      ToolConfirmationRequest
    >({
      queryFn: async (args, api, _extraOptions, baseQuery) => {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;

        const { messages, tool_calls } = args;
        const messagesForLsp = formatMessagesForLsp(messages);

        const url = `http://127.0.0.1:${port}${TOOLS_CHECK_CONFIRMATION}`;
        const result = await baseQuery({
          url,
          method: "POST",
          body: {
            tool_calls: tool_calls,
            messages: messagesForLsp,
          },
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) return result;
        if (!isToolConfirmationResponse(result.data)) {
          return {
            error: {
              error: "Invalid response from tools",
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

export type ToolParams = {
  name: string;
  type: string;
  description: string;
};

export type ToolFunction = {
  agentic?: boolean;
  name: string;
  description: string;
  // parameters: ToolParams[];
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
