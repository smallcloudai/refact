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
    // TODO: set this to the configured lsp url
    baseUrl: "http://127.0.0.1:8001",
  }),
  endpoints: (builder) => ({
    getCommandCompletion: builder.query<
      CommandCompletionResponse,
      CompletionArgs
    >({
      query: (args: CompletionArgs) => {
        return {
          url: AT_COMMAND_COMPLETION,
          method: "POST",
          body: {
            query: args.query,
            cursor: args.cursor,
            top_n: args.top_n ?? 5,
          },
        };
      },
      transformResponse: (response) => {
        if (
          !isCommandCompletionResponse(response) &&
          !isDetailMessage(response)
        ) {
          throw new Error("Invalid response from command completion");
        }

        if (isDetailMessage(response)) {
          return {
            completions: [],
            replace: [0, 0],
            is_cmd_executable: false,
          };
        }
        return response;
      },
    }),
    getCommandPreview: builder.query<ChatContextFile[], string>({
      query: (query) => {
        return {
          url: AT_COMMAND_PREVIEW,
          method: "POST",
          body: { query },
        };
      },
      transformResponse: (response) => {
        if (!isCommandPreviewResponse(response) && !isDetailMessage(response)) {
          throw new Error("Invalid response from command preview");
        }

        if (isDetailMessage(response)) {
          return [];
        }

        const files = response.messages.reduce<ChatContextFile[]>(
          (acc, { content }) => {
            const fileData = parseOrElse<ChatContextFile[]>(content, []);
            return [...acc, ...fileData];
          },
          [],
        );

        return files;
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

// export async function getAtCommandCompletion(
//   query: string,
//   cursor: number,
//   number: number,
//   lspUrl?: string,
// ): Promise<CommandCompletionResponse> {
//   const completionEndpoint = lspUrl
//     ? `${lspUrl.replace(/\/*$/, "")}${AT_COMMAND_COMPLETION}`
//     : AT_COMMAND_COMPLETION;

//   const response = await fetch(completionEndpoint, {
//     method: "POST",
//     headers: {
//       "Content-Type": "application/json",
//     },
//     body: JSON.stringify({ query, cursor, top_n: number }),
//   });

//   if (!response.ok) {
//     throw new Error(response.statusText);
//   }

//   const json: unknown = await response.json();
//   if (!isCommandCompletionResponse(json) && !isDetailMessage(json)) {
//     throw new Error("Invalid response from completion");
//   }

//   if (isDetailMessage(json)) {
//     return {
//       completions: [],
//       replace: [0, 0],
//       is_cmd_executable: false,
//     };
//   }

//   return json;
// }

export type CommandPreviewContent = {
  content: string;
  role: "context_file";
};
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

  const firstMessage: unknown = json.messages[0];
  if (!firstMessage) return false;
  if (typeof firstMessage !== "object") return false;
  if (!("role" in firstMessage)) return false;
  if (firstMessage.role !== "context_file") return false;
  if (!("content" in firstMessage)) return false;
  if (typeof firstMessage.content !== "string") return false;

  return true;
}

// export async function getAtCommandPreview(
//   query: string,
//   lspUrl?: string,
// ): Promise<ChatContextFileMessage[]> {
//   // check this
//   const previewEndpoint = lspUrl
//     ? `${lspUrl.replace(/\/*$/, "")}${AT_COMMAND_PREVIEW}`
//     : AT_COMMAND_PREVIEW;

//   const response = await fetch(previewEndpoint, {
//     method: "POST",
//     headers: {
//       "Content-Type": "application/json",
//     },
//     redirect: "follow",
//     cache: "no-cache",
//     referrer: "no-referrer",
//     credentials: "same-origin",
//     body: JSON.stringify({ query }),
//   });

//   if (!response.ok) {
//     throw new Error(response.statusText);
//   }

//   const json: unknown = await response.json();

//   if (!isCommandPreviewResponse(json) && !isDetailMessage(json)) {
//     throw new Error("Invalid response from command preview");
//   }

//   if (isDetailMessage(json)) {
//     return [];
//   }

//   const jsonMessages = json.messages.map<ChatContextFileMessage>(
//     ({ role, content }) => {
//       const fileData = JSON.parse(content) as ChatContextFile[];
//       return [role, fileData];
//     },
//   );

//   return jsonMessages;
// }
