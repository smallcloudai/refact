// import { getApiKey } from "../../utils/ApiKey";
import { RootState } from "../../app/store";
import { AT_TOOLS_AVAILABLE_URL } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

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
    getTools: builder.query<ToolCommand[], { port: number }>({
      query: ({ port }) => `http://127.0.0.1:${port}${AT_TOOLS_AVAILABLE_URL}`,
      transformResponse: (response) => {
        if (!Array.isArray(response)) {
          throw new Error("Invalid response from caps");
        }
        const tools: ToolCommand[] = response.filter((d) =>
          isToolCommand(d),
        ) as ToolCommand[];

        return tools;
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

// export async function getAvailableTools(
//   lspUrl?: string,
// ): Promise<ToolCommand[]> {
//   const toolsUrl = lspUrl
//     ? `${lspUrl.replace(/\/*$/, "")}${AT_TOOLS_AVAILABLE_URL}`
//     : AT_TOOLS_AVAILABLE_URL;

//   const apiKey = getApiKey();

//   const response = await fetch(toolsUrl, {
//     method: "GET",
//     credentials: "same-origin",
//     redirect: "follow",
//     cache: "no-cache",
//     referrer: "no-referrer",
//     headers: {
//       accept: "application/json",
//       ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
//     },
//   });

//   if (!response.ok) {
//     return [];
//   }

//   // TODO: add type guards
//   return (await response.json()) as unknown as ToolCommand[];
// }
