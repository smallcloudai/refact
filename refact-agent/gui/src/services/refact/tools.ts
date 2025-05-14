import { RootState } from "../../app/store";
import {
  TOOLS_CHECK_CONFIRMATION,
  EDIT_TOOL_DRY_RUN_URL,
  TOOLS,
} from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import {
  ChatMessage,
  DiffChunk,
  isDiffChunk,
  isSuccess,
  ToolCall,
} from "./types";
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
    getToolGroups: builder.query<ToolGroup[], undefined>({
      queryFn: async (_args, api, _extraOptions, baseQuery) => {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;
        const url = `http://127.0.0.1:${port}${TOOLS}`;
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
        const toolGroups = result.data.filter((d) =>
          isToolGroup(d),
        ) as ToolGroup[];
        return { data: toolGroups };
      },
    }),
    updateToolGroups: builder.mutation<{ success: true }, ToolGroupUpdate[]>({
      queryFn: async (newToolGroups, api, _extraOptions, baseQuery) => {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;
        const url = `http://127.0.0.1:${port}${TOOLS}`;
        const result = await baseQuery({
          method: "POST",
          url,
          body: JSON.stringify({
            tools: newToolGroups,
          }),
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) return result;
        if (!isSuccess(result.data)) {
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
    dryRunForEditTool: builder.mutation<
      ToolEditResult,
      { toolName: string; toolArgs: Record<string, unknown> }
    >({
      async queryFn(args, api, extraOptions, baseQuery) {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;
        const url = `http://127.0.0.1:${port}${EDIT_TOOL_DRY_RUN_URL}`;

        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: { tool_name: args.toolName, tool_args: args.toolArgs },
          credentials: "same-origin",
          redirect: "follow",
        });

        if (response.error) return response;

        if (!isToolEditResult(response.data)) {
          return {
            error: {
              error: `Invalid response from ${EDIT_TOOL_DRY_RUN_URL}`,
              data: response.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: response.data };
      },
    }),
  }),
  refetchOnMountOrArgChange: true,
});

export type ToolGroupUpdate = {
  name: string;
  source: ToolSource;
  enabled: boolean;
};

export type ToolGroup = {
  name: string;
  category: "integration" | "mcp" | "builtin";
  description: string;
  tools: Tool[];
};

export type ToolSource = {
  source_type: "builtin" | "integration";
  config_path: string;
};

export type ToolParam = {
  name: string;
  type: string;
  description: string;
};

export type ToolSpec = {
  name: string;
  display_name: string;
  description: string;

  // TODO: investigate on parameters
  parameters: ToolParam[];
  // parameters: Record<string, unknown>;
  source: ToolSource;

  parameters_required?: string[];
  agentic: boolean;
  experimental?: boolean;
};

export type Tool = {
  spec: ToolSpec;
  enabled: boolean;
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

export function isToolGroup(tool: unknown): tool is ToolGroup {
  if (!tool || typeof tool !== "object") return false;
  const group = tool as ToolGroup;
  if (typeof group.name !== "string") return false;
  if (typeof group.category !== "string") return false;
  if (typeof group.description !== "string") return false;
  if (!Array.isArray(group.tools)) return false;
  // Optionally, check that every element in tools is a Tool (if you have isTool)
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
