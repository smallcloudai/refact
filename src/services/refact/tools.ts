// import { getApiKey } from "../../utils/ApiKey";
import { RootState } from "../../app/store";
import { AT_TOOLS_AVAILABLE_URL } from "./consts";
import {
  createApi,
  fetchBaseQuery,
  FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";

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
        const result = await baseQuery(url);
        if (result.error) return result;
        if (!Array.isArray(result.data)) {
          return {
            error: {
              error: "Invalid response from tools",
              data: result.data,
            } as FetchBaseQueryError,
          };
        }
        const tools = result.data.filter((d) =>
          isToolCommand(d),
        ) as ToolCommand[];
        return { data: tools };
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
  parameters: ToolParams[];
  parameters_required: string[];
};

export type ToolCommand = {
  function: ToolFunction;
  type: "function";
};

function isToolCommand(tool: unknown): tool is ToolCommand {
  if (!tool) return false;
  if (typeof tool !== "object") return false;
  if (!("type" in tool) || !("function" in tool)) return false;
  return true;
}
